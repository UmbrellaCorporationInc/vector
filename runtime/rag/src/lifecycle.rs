//! `LanceDB` Phase 6 lifecycle boundary for the local RAG store.

use crate::{LANCEDB_PRIMARY_CHUNK_TABLE, RagDefaults};
use lancedb::{
    Connection,
    arrow::arrow_schema::{DataType, Field, Schema, SchemaRef},
    connect,
    index::Index,
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

/// Input contract for creating or validating the local `LanceDB` store.
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
                write!(formatter, "failed to create LanceDB full-text index for '{table}': {message}")
            }
        }
    }
}

impl std::error::Error for LanceDbStoreError {}

/// Create or validate the Phase 6 `LanceDB` store under the governed RAG path.
///
/// The operation keeps all schema and index creation behind `runtime-rag` so
/// CLI callers do not own any LanceDB-specific lifecycle rules.
pub async fn ensure_lancedb_store(
    request: &LanceDbStoreRequest,
) -> Result<LanceDbStoreStatus, LanceDbStoreError> {
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

    let database_dir = request
        .root_dir
        .join(RagDefaults::phase_one().lancedb_storage_path());
    std::fs::create_dir_all(&database_dir).map_err(|error| {
        LanceDbStoreError::CreateStoreDirectory {
            path: database_dir.clone(),
            message: error.to_string(),
        }
    })?;

    let database = connect(database_dir.to_string_lossy().as_ref())
        .execute()
        .await
        .map_err(|error| LanceDbStoreError::ConnectDatabase {
            path: database_dir.clone(),
            message: error.to_string(),
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

async fn ensure_primary_table(
    database: &Connection,
    request: &LanceDbStoreRequest,
) -> Result<(lancedb::Table, bool), LanceDbStoreError> {
    let table_name = LANCEDB_PRIMARY_CHUNK_TABLE.to_owned();
    match database.open_table(table_name.clone()).execute().await {
        Ok(table) => Ok((table, false)),
        Err(_) => {
            database
                .create_empty_table(table_name.clone(), primary_table_schema(request)?)
                .execute()
                .await
                .map_err(|error| LanceDbStoreError::CreatePrimaryTable {
                    table: table_name.clone(),
                    message: error.to_string(),
                })?;

            let table =
                database.open_table(table_name.clone()).execute().await.map_err(|error| {
                    LanceDbStoreError::OpenPrimaryTable {
                        table: table_name.clone(),
                        message: error.to_string(),
                    }
                })?;
            Ok((table, true))
        }
    }
}

fn primary_table_schema(
    request: &LanceDbStoreRequest,
) -> Result<SchemaRef, LanceDbStoreError> {
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
        (
            STORE_METADATA_SCHEMA_VERSION_KEY.to_owned(),
            STORE_SCHEMA_VERSION.to_owned(),
        ),
        (
            STORE_METADATA_EMBEDDING_MODEL_KEY.to_owned(),
            request.embedding_model.clone(),
        ),
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
            Field::new("heading_path", DataType::List(list_item_field.clone()), false),
            Field::new("frontmatter", DataType::Utf8, true),
            Field::new("text", DataType::Utf8, false),
            Field::new("token_count", DataType::UInt32, false),
            Field::new("embedding_model", DataType::Utf8, false),
            Field::new("embedding_dimension", DataType::UInt32, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(vector_field, dimension),
                false,
            ),
        ],
        metadata,
    )))
}

async fn validate_table_contract(
    table: &lancedb::Table,
    request: &LanceDbStoreRequest,
) -> Result<(), LanceDbStoreError> {
    let table_name = table.name().to_owned();
    let schema = table
        .schema()
        .await
        .map_err(|error| LanceDbStoreError::OpenPrimaryTable {
            table: table_name.clone(),
            message: error.to_string(),
        })?;

    let metadata = schema.metadata();
    let actual_model = metadata
        .get(STORE_METADATA_EMBEDDING_MODEL_KEY)
        .cloned()
        .unwrap_or_default();
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

    validate_vector_field(&schema, request.embedding_dimension, &table.name().to_owned())
}

fn validate_vector_field(
    schema: &Schema,
    expected_dimension: usize,
    table_name: &str,
) -> Result<(), LanceDbStoreError> {
    let field = schema
        .field_with_name("vector")
        .map_err(|error| LanceDbStoreError::InvalidVectorColumn {
            table: table_name.to_owned(),
            message: error.to_string(),
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
    match table
        .create_index(&["text"], Index::FTS(Default::default()))
        .execute()
        .await
    {
        Ok(()) => Ok(true),
        Err(error) if already_exists_error(&error.to_string()) => Ok(false),
        Err(error) => Err(LanceDbStoreError::CreateTextIndex {
            table: table.name().to_owned(),
            message: error.to_string(),
        }),
    }
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
