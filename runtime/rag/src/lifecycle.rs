//! `LanceDB` Phase 6 lifecycle boundary for the local RAG store.

use crate::{
    EmbeddedMarkdownChunkBatch, LANCEDB_PRIMARY_CHUNK_TABLE, LanceDbChunkRow, RagDefaults,
    lancedb_chunk_row,
};
use arrow_array::builder::StringBuilder;
use arrow_array::types::Float32Type;
use arrow_array::{
    ArrayRef, FixedSizeListArray, ListArray, RecordBatch, RecordBatchIterator, StringArray,
    UInt32Array,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use lancedb::{
    Connection, Table, connect,
    index::{Index, scalar::FtsIndexBuilder, vector::IvfFlatIndexBuilder},
};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

const STORE_SCHEMA_VERSION: &str = "phase-6-v1";
const STORE_METADATA_SCHEMA_VERSION_KEY: &str = "vector.rag.store_schema_version";
const STORE_METADATA_EMBEDDING_MODEL_KEY: &str = "vector.rag.embedding_model";
const STORE_METADATA_EMBEDDING_DIMENSION_KEY: &str = "vector.rag.embedding_dimension";
type PrimaryTableState = (Table, bool);

/// Input contract for creating or validating the local `LanceDB` store.
///
/// # DTO(storage lifecycle request shared by CLI and indexing callers)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbStoreRequest {
    /// Workspace root used to resolve the Phase 1 `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Embedding model identifier that owns the active store contract.
    pub embedding_model: String,
    /// Embedding dimension that owns the active store contract.
    pub embedding_dimension: usize,
}

/// Outcome for the high-level `LanceDB` lifecycle operation.
///
/// # DTO(storage lifecycle status returned to callers that orchestrate Phase 6 setup)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbStoreStatus {
    /// Resolved local database directory under `.vector-database/rag/lancedb/`.
    pub database_dir: PathBuf,
    /// Primary Phase 6 chunk table name.
    pub table_name: String,
    /// Whether the primary table was created during this call.
    pub created_table: bool,
    /// Whether the full-text index was created during this call.
    pub created_text_index: bool,
}

/// Input contract for persisting one embedded Markdown document batch.
///
/// # DTO(indexing write request shared between pipeline orchestration and persistence)
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LanceDbChunkWriteRequest {
    /// Workspace root used to resolve the Phase 1 `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Embedded chunk batch for exactly one governed document.
    pub batch: EmbeddedMarkdownChunkBatch,
}

/// Outcome for persisting one embedded Markdown document batch.
///
/// # DTO(indexing write status returned after deterministic `LanceDB` persistence)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbChunkWriteStatus {
    /// Resolved local database directory under `.vector-database/rag/lancedb/`.
    pub database_dir: PathBuf,
    /// Primary Phase 6 chunk table name.
    pub table_name: String,
    /// Merge commit version returned by `LanceDB`.
    pub version: u64,
    /// Number of rows inserted by the merge operation.
    pub inserted_rows: u64,
    /// Number of rows updated by the merge operation.
    pub updated_rows: u64,
    /// Number of stale rows deleted by the merge operation.
    pub deleted_rows: u64,
    /// Whether the vector index was created during this call.
    pub created_vector_index: bool,
}

/// Input contract for deleting all persisted rows for one governed document.
///
/// # DTO(document delete request shared between index reconciliation callers and persistence)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbDocumentDeleteRequest {
    /// Workspace root used to resolve the Phase 1 `LanceDB` storage path.
    pub root_dir: PathBuf,
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Embedding model identifier that owns the active store contract.
    pub embedding_model: String,
    /// Embedding dimension that owns the active store contract.
    pub embedding_dimension: usize,
}

/// Outcome for deleting one governed document from the local `LanceDB` store.
///
/// # DTO(document delete status returned after stale-row removal)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanceDbDocumentDeleteStatus {
    /// Resolved local database directory under `.vector-database/rag/lancedb/`.
    pub database_dir: PathBuf,
    /// Primary Phase 6 chunk table name.
    pub table_name: String,
    /// Number of rows deleted for the governed document.
    pub deleted_rows: u64,
}

/// Actionable `LanceDB` store lifecycle failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanceDbStoreError {
    /// The requested store contract is invalid before any storage operation runs.
    InvalidRequest {
        /// Human-readable validation failure.
        message: String,
    },
    /// The local store directory could not be prepared.
    CreateStoreDirectory {
        /// Target local store path.
        path: PathBuf,
        /// Human-readable filesystem failure.
        message: String,
    },
    /// Connecting to `LanceDB` failed.
    ConnectDatabase {
        /// Target local database path.
        path: PathBuf,
        /// Human-readable connection failure.
        message: String,
    },
    /// Creating the primary table failed.
    CreatePrimaryTable {
        /// Primary table name.
        table: String,
        /// Human-readable create-table failure.
        message: String,
    },
    /// Opening the primary table failed.
    OpenPrimaryTable {
        /// Primary table name.
        table: String,
        /// Human-readable open-table failure.
        message: String,
    },
    /// The existing table contract does not match the active embedding contract.
    IncompatibleStoreContract {
        /// Primary table name.
        table: String,
        /// Requested embedding model identifier.
        expected_embedding_model: String,
        /// Existing embedding model identifier stored in metadata.
        actual_embedding_model: String,
        /// Requested embedding dimension.
        expected_embedding_dimension: usize,
        /// Existing embedding dimension stored in metadata.
        actual_embedding_dimension: usize,
    },
    /// The existing table schema does not expose the required vector field shape.
    InvalidVectorColumn {
        /// Primary table name.
        table: String,
        /// Human-readable schema validation failure.
        message: String,
    },
    /// Creating the required full-text index failed.
    CreateTextIndex {
        /// Primary table name.
        table: String,
        /// Human-readable index creation failure.
        message: String,
    },
    /// Persisting embedded rows failed.
    PersistRows {
        /// Primary table name.
        table: String,
        /// Human-readable merge failure.
        message: String,
    },
    /// Deleting persisted rows for one document failed.
    DeleteDocumentRows {
        /// Primary table name.
        table: String,
        /// Human-readable delete failure.
        message: String,
    },
    /// Creating the required vector index failed.
    CreateVectorIndex {
        /// Primary table name.
        table: String,
        /// Human-readable index creation failure.
        message: String,
    },
}

impl Display for LanceDbStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest { message } => formatter.write_str(message),
            Self::CreateStoreDirectory { path, message } => {
                write!(
                    formatter,
                    "failed to create LanceDB store directory '{}': {message}",
                    path.display()
                )
            }
            Self::ConnectDatabase { path, message } => {
                write!(
                    formatter,
                    "failed to connect LanceDB database at '{}': {message}",
                    path.display()
                )
            }
            Self::CreatePrimaryTable { table, message } => {
                write!(formatter, "failed to create LanceDB table '{table}': {message}")
            }
            Self::OpenPrimaryTable { table, message } => {
                write!(formatter, "failed to open LanceDB table '{table}': {message}")
            }
            Self::IncompatibleStoreContract {
                table,
                expected_embedding_model,
                actual_embedding_model,
                expected_embedding_dimension,
                actual_embedding_dimension,
            } => write!(
                formatter,
                "LanceDB table '{table}' is incompatible with embedding contract: expected model '{expected_embedding_model}' and dimension {expected_embedding_dimension}, found model '{actual_embedding_model}' and dimension {actual_embedding_dimension}"
            ),
            Self::InvalidVectorColumn { table, message } => {
                write!(formatter, "LanceDB table '{table}' has an invalid vector column: {message}")
            }
            Self::CreateTextIndex { table, message } => {
                write!(
                    formatter,
                    "failed to create LanceDB full-text index for '{table}': {message}"
                )
            }
            Self::PersistRows { table, message } => {
                write!(formatter, "failed to persist LanceDB rows for '{table}': {message}")
            }
            Self::DeleteDocumentRows { table, message } => {
                write!(formatter, "failed to delete LanceDB rows for '{table}': {message}")
            }
            Self::CreateVectorIndex { table, message } => {
                write!(formatter, "failed to create LanceDB vector index for '{table}': {message}")
            }
        }
    }
}

impl std::error::Error for LanceDbStoreError {}

/// Create or validate the Phase 6 `LanceDB` store under the governed RAG path.
///
/// The operation keeps all schema and index creation behind `runtime-rag` so
/// CLI callers do not own any LanceDB-specific lifecycle rules.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the request is invalid, the store cannot
/// be prepared, or the existing store contract is incompatible.
pub async fn ensure_lancedb_store(
    request: &LanceDbStoreRequest,
) -> Result<LanceDbStoreStatus, LanceDbStoreError> {
    validate_store_request(request)?;

    let database_dir = request.root_dir.join(RagDefaults::phase_one().lancedb_storage_path());
    std::fs::create_dir_all(&database_dir).map_err(|error| {
        LanceDbStoreError::CreateStoreDirectory {
            path: database_dir.clone(),
            message: error.to_string(),
        }
    })?;

    let database =
        connect(database_dir.to_string_lossy().as_ref()).execute().await.map_err(|error| {
            LanceDbStoreError::ConnectDatabase {
                path: database_dir.clone(),
                message: error.to_string(),
            }
        })?;

    let (table, created_table) = ensure_primary_table(&database, request).await?;
    validate_table_contract(&table, request).await?;
    let created_text_index = ensure_text_index(&table).await?;

    Ok(LanceDbStoreStatus {
        database_dir,
        table_name: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
        created_table,
        created_text_index,
    })
}

/// Persist one embedded Markdown document batch into the Phase 6 `LanceDB` store.
///
/// The operation upserts rows by stable `chunk_id`, deletes stale rows for the
/// same document identity, and creates the vector index only after persisted
/// rows exist.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the batch is malformed, the active store
/// contract is incompatible, or the write/index operations fail.
pub async fn persist_embedded_markdown_chunks(
    request: &LanceDbChunkWriteRequest,
) -> Result<LanceDbChunkWriteStatus, LanceDbStoreError> {
    let rows = validate_chunk_write_request(request)?;
    let first_row = rows.first().ok_or_else(|| LanceDbStoreError::InvalidRequest {
        message: "embedded chunk batch must contain at least one chunk".to_owned(),
    })?;
    let store_request = LanceDbStoreRequest {
        root_dir: request.root_dir.clone(),
        embedding_model: first_row.embedding_model.clone(),
        embedding_dimension: first_row.embedding_dimension,
    };
    let store_status = ensure_lancedb_store(&store_request).await?;
    let table = open_primary_table(&store_status.database_dir).await?;
    let merge_result = persist_rows(&table, &rows).await?;
    let created_vector_index = ensure_vector_index_if_needed(&table).await?;

    Ok(LanceDbChunkWriteStatus {
        database_dir: store_status.database_dir,
        table_name: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
        version: merge_result.version,
        inserted_rows: merge_result.num_inserted_rows,
        updated_rows: merge_result.num_updated_rows,
        deleted_rows: merge_result.num_deleted_rows,
        created_vector_index,
    })
}

/// Delete all persisted rows for one governed document from the Phase 6 store.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the active store contract is incompatible
/// or the delete operation fails.
pub async fn delete_document_chunks(
    request: &LanceDbDocumentDeleteRequest,
) -> Result<LanceDbDocumentDeleteStatus, LanceDbStoreError> {
    let store_request = LanceDbStoreRequest {
        root_dir: request.root_dir.clone(),
        embedding_model: request.embedding_model.clone(),
        embedding_dimension: request.embedding_dimension,
    };
    let store_status = ensure_lancedb_store(&store_request).await?;
    let table = open_primary_table(&store_status.database_dir).await?;
    let deleted_rows =
        delete_document_rows(&table, request.package.as_deref(), &request.document_stem).await?;

    Ok(LanceDbDocumentDeleteStatus {
        database_dir: store_status.database_dir,
        table_name: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
        deleted_rows,
    })
}

async fn ensure_primary_table(
    database: &Connection,
    request: &LanceDbStoreRequest,
) -> Result<PrimaryTableState, LanceDbStoreError> {
    let table_name = LANCEDB_PRIMARY_CHUNK_TABLE.to_owned();
    if let Ok(table) = database.open_table(table_name.clone()).execute().await {
        return Ok((table, false));
    }

    database
        .create_empty_table(table_name.clone(), primary_table_schema(request)?)
        .execute()
        .await
        .map_err(|error| LanceDbStoreError::CreatePrimaryTable {
            table: table_name.clone(),
            message: error.to_string(),
        })?;

    let table = database.open_table(table_name.clone()).execute().await.map_err(|error| {
        LanceDbStoreError::OpenPrimaryTable {
            table: table_name.clone(),
            message: error.to_string(),
        }
    })?;
    Ok((table, true))
}

fn validate_store_request(request: &LanceDbStoreRequest) -> Result<(), LanceDbStoreError> {
    if request.embedding_model.trim().is_empty() {
        return Err(LanceDbStoreError::InvalidRequest {
            message: "embedding_model must not be empty".to_owned(),
        });
    }
    if request.embedding_dimension == 0 {
        return Err(LanceDbStoreError::InvalidRequest {
            message: "embedding_dimension must be greater than zero".to_owned(),
        });
    }

    Ok(())
}

fn primary_table_schema(request: &LanceDbStoreRequest) -> Result<SchemaRef, LanceDbStoreError> {
    let dimension = i32::try_from(request.embedding_dimension).map_err(|_| {
        LanceDbStoreError::InvalidRequest {
            message: format!(
                "embedding_dimension {} exceeds LanceDB fixed-size list limits",
                request.embedding_dimension
            ),
        }
    })?;
    let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
    let list_item_field = Arc::new(Field::new("item", DataType::Utf8, true));
    let metadata = HashMap::from([
        (STORE_METADATA_SCHEMA_VERSION_KEY.to_owned(), STORE_SCHEMA_VERSION.to_owned()),
        (STORE_METADATA_EMBEDDING_MODEL_KEY.to_owned(), request.embedding_model.clone()),
        (
            STORE_METADATA_EMBEDDING_DIMENSION_KEY.to_owned(),
            request.embedding_dimension.to_string(),
        ),
    ]);

    Ok(Arc::new(Schema::new_with_metadata(
        vec![
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("package", DataType::Utf8, true),
            Field::new("document_stem", DataType::Utf8, false),
            Field::new("document_hash", DataType::Utf8, false),
            Field::new("chunk_hash", DataType::Utf8, false),
            Field::new("chunk_ordinal", DataType::UInt32, false),
            Field::new("heading_path", DataType::List(list_item_field), false),
            Field::new("frontmatter", DataType::Utf8, true),
            Field::new("text", DataType::Utf8, false),
            Field::new("token_count", DataType::UInt32, false),
            Field::new("embedding_model", DataType::Utf8, false),
            Field::new("embedding_dimension", DataType::UInt32, false),
            Field::new("vector", DataType::FixedSizeList(vector_field, dimension), false),
        ],
        metadata,
    )))
}

async fn validate_table_contract(
    table: &lancedb::Table,
    request: &LanceDbStoreRequest,
) -> Result<(), LanceDbStoreError> {
    let table_name = table.name().to_owned();
    let schema = table.schema().await.map_err(|error| LanceDbStoreError::OpenPrimaryTable {
        table: table_name.clone(),
        message: error.to_string(),
    })?;

    let metadata = schema.metadata();
    let actual_model =
        metadata.get(STORE_METADATA_EMBEDDING_MODEL_KEY).cloned().unwrap_or_default();
    let actual_dimension = metadata
        .get(STORE_METADATA_EMBEDDING_DIMENSION_KEY)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default();
    if actual_model != request.embedding_model || actual_dimension != request.embedding_dimension {
        return Err(LanceDbStoreError::IncompatibleStoreContract {
            table: table_name,
            expected_embedding_model: request.embedding_model.clone(),
            actual_embedding_model: actual_model,
            expected_embedding_dimension: request.embedding_dimension,
            actual_embedding_dimension: actual_dimension,
        });
    }

    validate_vector_field(&schema, request.embedding_dimension, table.name())
}

fn validate_vector_field(
    schema: &Schema,
    expected_dimension: usize,
    table_name: &str,
) -> Result<(), LanceDbStoreError> {
    let field = schema.field_with_name("vector").map_err(|error| {
        LanceDbStoreError::InvalidVectorColumn {
            table: table_name.to_owned(),
            message: error.to_string(),
        }
    })?;

    match field.data_type() {
        DataType::FixedSizeList(item, dimension) => {
            if item.data_type() != &DataType::Float32 {
                return Err(LanceDbStoreError::InvalidVectorColumn {
                    table: table_name.to_owned(),
                    message: "vector items must be Float32".to_owned(),
                });
            }
            let actual_dimension = usize::try_from(*dimension).unwrap_or_default();
            if actual_dimension != expected_dimension {
                return Err(LanceDbStoreError::IncompatibleStoreContract {
                    table: table_name.to_owned(),
                    expected_embedding_model: schema
                        .metadata()
                        .get(STORE_METADATA_EMBEDDING_MODEL_KEY)
                        .cloned()
                        .unwrap_or_default(),
                    actual_embedding_model: schema
                        .metadata()
                        .get(STORE_METADATA_EMBEDDING_MODEL_KEY)
                        .cloned()
                        .unwrap_or_default(),
                    expected_embedding_dimension: expected_dimension,
                    actual_embedding_dimension: actual_dimension,
                });
            }
            Ok(())
        }
        other => Err(LanceDbStoreError::InvalidVectorColumn {
            table: table_name.to_owned(),
            message: format!("expected FixedSizeList(Float32, N) but found {other:?}"),
        }),
    }
}

async fn ensure_text_index(table: &lancedb::Table) -> Result<bool, LanceDbStoreError> {
    match table.create_index(&["text"], Index::FTS(FtsIndexBuilder::default())).execute().await {
        Ok(()) => Ok(true),
        Err(error) if already_exists_error(&error.to_string()) => Ok(false),
        Err(error) => Err(LanceDbStoreError::CreateTextIndex {
            table: table.name().to_owned(),
            message: error.to_string(),
        }),
    }
}

async fn open_primary_table(database_dir: &Path) -> Result<Table, LanceDbStoreError> {
    let database =
        connect(database_dir.to_string_lossy().as_ref()).execute().await.map_err(|error| {
            LanceDbStoreError::ConnectDatabase {
                path: database_dir.to_path_buf(),
                message: error.to_string(),
            }
        })?;

    database.open_table(LANCEDB_PRIMARY_CHUNK_TABLE).execute().await.map_err(|error| {
        LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        }
    })
}

fn validate_chunk_write_request(
    request: &LanceDbChunkWriteRequest,
) -> Result<Vec<LanceDbChunkRow>, LanceDbStoreError> {
    let extraction = &request.batch.extraction;
    if request.batch.chunks.is_empty() {
        return Err(LanceDbStoreError::InvalidRequest {
            message: "embedded chunk batch must contain at least one chunk".to_owned(),
        });
    }
    if request.batch.package != extraction.package
        || request.batch.document_stem != extraction.document_stem
        || request.batch.document_hash != extraction.document_hash
    {
        return Err(LanceDbStoreError::InvalidRequest {
            message: "embedded chunk batch identity must match the normalized extraction record"
                .to_owned(),
        });
    }

    let mut rows = Vec::with_capacity(request.batch.chunks.len());
    let mut expected_model = None::<&str>;
    let mut expected_dimension = None::<usize>;

    for chunk in &request.batch.chunks {
        if chunk.chunk.package != request.batch.package
            || chunk.chunk.document_stem != request.batch.document_stem
            || chunk.chunk.document_hash != request.batch.document_hash
        {
            return Err(LanceDbStoreError::InvalidRequest {
                message: format!(
                    "chunk '{}' does not match the batch document identity",
                    chunk.chunk.chunk_id
                ),
            });
        }

        let model = chunk.embedding_model.as_str();
        let dimension = chunk.embedding_dimension;
        if expected_model.is_none() {
            expected_model = Some(model);
            expected_dimension = Some(dimension);
        } else if expected_model != Some(model) || expected_dimension != Some(dimension) {
            return Err(LanceDbStoreError::InvalidRequest {
                message: format!(
                    "chunk '{}' uses embedding contract '{}'/{dimension}, which does not match the rest of the batch",
                    chunk.chunk.chunk_id, chunk.embedding_model
                ),
            });
        }

        rows.push(lancedb_chunk_row(extraction, chunk));
    }

    Ok(rows)
}

async fn persist_rows(
    table: &Table,
    rows: &[LanceDbChunkRow],
) -> Result<lancedb::table::MergeResult, LanceDbStoreError> {
    let mut merge = table.merge_insert(&["chunk_id"]);
    merge
        .when_matched_update_all(None)
        .when_not_matched_insert_all()
        .when_not_matched_by_source_delete(Some(document_predicate(
            rows[0].package.as_deref(),
            &rows[0].document_stem,
            None,
        )));

    merge.execute(chunk_rows_reader(rows)?).await.map_err(|error| LanceDbStoreError::PersistRows {
        table: table.name().to_owned(),
        message: error.to_string(),
    })
}

async fn ensure_vector_index_if_needed(table: &Table) -> Result<bool, LanceDbStoreError> {
    if table.count_rows(None).await.map_err(|error| LanceDbStoreError::OpenPrimaryTable {
        table: table.name().to_owned(),
        message: error.to_string(),
    })? == 0
    {
        return Ok(false);
    }

    match table
        .create_index(&["vector"], Index::IvfFlat(IvfFlatIndexBuilder::default()))
        .replace(false)
        .execute()
        .await
    {
        Ok(()) => Ok(true),
        Err(error) if already_exists_error(&error.to_string()) => Ok(false),
        Err(error) => Err(LanceDbStoreError::CreateVectorIndex {
            table: table.name().to_owned(),
            message: error.to_string(),
        }),
    }
}

async fn delete_document_rows(
    table: &Table,
    package: Option<&str>,
    document_stem: &str,
) -> Result<u64, LanceDbStoreError> {
    let predicate = document_predicate(package, document_stem, None);
    let before = table.count_rows(Some(predicate.clone())).await.map_err(|error| {
        LanceDbStoreError::DeleteDocumentRows {
            table: table.name().to_owned(),
            message: error.to_string(),
        }
    })?;
    table.delete(&predicate).await.map_err(|error| LanceDbStoreError::DeleteDocumentRows {
        table: table.name().to_owned(),
        message: error.to_string(),
    })?;
    Ok(u64::try_from(before).unwrap_or(u64::MAX))
}

fn chunk_rows_reader(
    rows: &[LanceDbChunkRow],
) -> Result<Box<dyn arrow_array::RecordBatchReader + Send>, LanceDbStoreError> {
    let batch = RecordBatch::try_new(
        primary_row_batch_schema(rows)?,
        vec![
            Arc::new(StringArray::from_iter_values(rows.iter().map(|row| row.chunk_id.as_str())))
                as ArrayRef,
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.package.as_deref()).collect::<Vec<_>>(),
            )) as ArrayRef,
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.document_stem.as_str()),
            )) as ArrayRef,
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.document_hash.as_str()),
            )) as ArrayRef,
            Arc::new(StringArray::from_iter_values(rows.iter().map(|row| row.chunk_hash.as_str())))
                as ArrayRef,
            Arc::new(UInt32Array::from(
                rows.iter().map(|row| u32::try_from(row.chunk_ordinal).ok()).collect::<Vec<_>>(),
            )) as ArrayRef,
            Arc::new(ListArray::from_nested_iter::<StringBuilder, _, _, _>(rows.iter().map(
                |row| {
                    Some(
                        row.heading_path
                            .iter()
                            .map(|segment| Some(segment.as_str()))
                            .collect::<Vec<_>>(),
                    )
                },
            ))) as ArrayRef,
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| {
                        row.frontmatter.as_ref().map(serde_json::to_string).transpose().map_err(
                            |error| LanceDbStoreError::InvalidRequest {
                                message: format!(
                                    "failed to serialize frontmatter for chunk '{}': {error}",
                                    row.chunk_id
                                ),
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )) as ArrayRef,
            Arc::new(StringArray::from_iter_values(rows.iter().map(|row| row.text.as_str())))
                as ArrayRef,
            Arc::new(UInt32Array::from(
                rows.iter().map(|row| u32::try_from(row.token_count).ok()).collect::<Vec<_>>(),
            )) as ArrayRef,
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.embedding_model.as_str()),
            )) as ArrayRef,
            Arc::new(UInt32Array::from(
                rows.iter()
                    .map(|row| u32::try_from(row.embedding_dimension).ok())
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
            Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                rows.iter()
                    .map(|row| Some(row.vector.iter().copied().map(Some).collect::<Vec<_>>())),
                i32::try_from(rows[0].embedding_dimension).map_err(|_| {
                    LanceDbStoreError::InvalidRequest {
                        message: format!(
                            "embedding_dimension {} exceeds LanceDB fixed-size list limits",
                            rows[0].embedding_dimension
                        ),
                    }
                })?,
            )) as ArrayRef,
        ],
    )
    .map_err(|error| LanceDbStoreError::InvalidRequest {
        message: format!("failed to build LanceDB write batch: {error}"),
    })?;

    let schema = batch.schema();
    Ok(Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema)))
}

fn primary_row_batch_schema(rows: &[LanceDbChunkRow]) -> Result<SchemaRef, LanceDbStoreError> {
    let dimension = i32::try_from(rows[0].embedding_dimension).map_err(|_| {
        LanceDbStoreError::InvalidRequest {
            message: format!(
                "embedding_dimension {} exceeds LanceDB fixed-size list limits",
                rows[0].embedding_dimension
            ),
        }
    })?;
    let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
    let list_item_field = Arc::new(Field::new("item", DataType::Utf8, true));

    Ok(Arc::new(Schema::new(vec![
        Field::new("chunk_id", DataType::Utf8, false),
        Field::new("package", DataType::Utf8, true),
        Field::new("document_stem", DataType::Utf8, false),
        Field::new("document_hash", DataType::Utf8, false),
        Field::new("chunk_hash", DataType::Utf8, false),
        Field::new("chunk_ordinal", DataType::UInt32, false),
        Field::new("heading_path", DataType::List(list_item_field), false),
        Field::new("frontmatter", DataType::Utf8, true),
        Field::new("text", DataType::Utf8, false),
        Field::new("token_count", DataType::UInt32, false),
        Field::new("embedding_model", DataType::Utf8, false),
        Field::new("embedding_dimension", DataType::UInt32, false),
        Field::new("vector", DataType::FixedSizeList(vector_field, dimension), false),
    ])))
}

fn document_predicate(package: Option<&str>, document_stem: &str, alias: Option<&str>) -> String {
    let package_column = qualified_column("package", alias);
    let stem_column = qualified_column("document_stem", alias);
    let package_predicate = package.map_or_else(
        || format!("{package_column} IS NULL"),
        |package| format!("{package_column} = '{}'", sql_string_literal(package)),
    );
    format!("{package_predicate} AND {stem_column} = '{}'", sql_string_literal(document_stem))
}

fn qualified_column(column: &str, alias: Option<&str>) -> String {
    alias.map_or_else(|| column.to_owned(), |alias| format!("{alias}.{column}"))
}

fn sql_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn already_exists_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("already exists") || normalized.contains("existing index")
}

/// Resolve the governed `LanceDB` store directory for a workspace root.
#[must_use]
pub fn lancedb_store_dir(root_dir: &Path) -> PathBuf {
    root_dir.join(RagDefaults::phase_one().lancedb_storage_path())
}

#[cfg(test)]
#[path = "lifecycle_test.rs"]
mod tests;
