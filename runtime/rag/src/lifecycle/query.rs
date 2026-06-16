//! Phase 7 incremental indexing reconciliation queries against the local `LanceDB` store.

use super::{
    LanceDbStoreError, delete_document_rows, document_predicate, open_primary_table,
    sql_string_literal,
};
use crate::{EmbeddingVector, LANCEDB_PRIMARY_CHUNK_TABLE, StoredChunkEmbeddings};
use arrow_array::FixedSizeListArray;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::path::Path;

/// Check whether a document with the given content hash is already indexed in the store.
///
/// Returns `true` when at least one row for `(package, document_stem, document_hash)` exists.
/// Returns `false` when no rows exist or when the store directory has not been created yet.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the table cannot be opened.
pub async fn query_document_hash_indexed(
    store_dir: &Path,
    package: Option<&str>,
    document_stem: &str,
    document_hash: &str,
) -> Result<bool, LanceDbStoreError> {
    if !store_dir.exists() {
        return Ok(false);
    }
    let table = match open_primary_table(store_dir).await {
        Ok(table) => table,
        Err(LanceDbStoreError::OpenPrimaryTable { .. }) => return Ok(false),
        Err(error) => return Err(error),
    };
    let predicate = format!(
        "{} AND document_hash = '{}'",
        document_predicate(package, document_stem, None),
        sql_string_literal(document_hash)
    );
    let count = table.count_rows(Some(predicate)).await.map_err(|error| {
        LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        }
    })?;
    Ok(count > 0)
}

/// Query stored chunk hashes and their embedding vectors for a governed document.
///
/// Returns a map from `chunk_hash` to its stored embedding vector. The caller uses
/// this to skip re-embedding chunks whose text and structural metadata are unchanged.
/// Returns an empty map when the store directory has not been created yet.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the table cannot be queried.
pub async fn query_document_chunk_embeddings(
    store_dir: &Path,
    package: Option<&str>,
    document_stem: &str,
) -> Result<StoredChunkEmbeddings, LanceDbStoreError> {
    use arrow_array::cast::AsArray;
    use futures::TryStreamExt;

    if !store_dir.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let table = match open_primary_table(store_dir).await {
        Ok(table) => table,
        Err(LanceDbStoreError::OpenPrimaryTable { .. }) => {
            return Ok(std::collections::HashMap::new());
        }
        Err(error) => return Err(error),
    };
    let predicate = document_predicate(package, document_stem, None);
    let stream = table
        .query()
        .only_if(predicate)
        .select(lancedb::query::Select::columns(&["chunk_hash", "vector"]))
        .execute()
        .await
        .map_err(|error| LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        })?;

    let batches: Vec<arrow_array::RecordBatch> =
        stream.try_collect().await.map_err(|error| LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        })?;

    let mut result = std::collections::HashMap::new();
    for batch in &batches {
        let chunk_hash_col = batch.column_by_name("chunk_hash").ok_or_else(|| {
            LanceDbStoreError::OpenPrimaryTable {
                table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                message: "chunk_hash column missing in query result".to_owned(),
            }
        })?;
        let vector_col =
            batch.column_by_name("vector").ok_or_else(|| LanceDbStoreError::OpenPrimaryTable {
                table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                message: "vector column missing in query result".to_owned(),
            })?;

        let chunk_hashes = chunk_hash_col.as_string::<i32>();
        let vectors =
            vector_col.as_any().downcast_ref::<FixedSizeListArray>().ok_or_else(|| {
                LanceDbStoreError::OpenPrimaryTable {
                    table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                    message: "vector column is not FixedSizeListArray".to_owned(),
                }
            })?;

        for row in 0..batch.num_rows() {
            let chunk_hash = chunk_hashes.value(row).to_owned();
            let slot = vectors.value(row);
            let floats =
                slot.as_any().downcast_ref::<arrow_array::Float32Array>().ok_or_else(|| {
                    LanceDbStoreError::OpenPrimaryTable {
                        table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                        message: "vector slot is not Float32Array".to_owned(),
                    }
                })?;
            let embedding: EmbeddingVector = (0..floats.len()).map(|i| floats.value(i)).collect();
            result.entry(chunk_hash).or_insert(embedding);
        }
    }
    Ok(result)
}

/// Resolved `(package, document_stem)` identity for one governed document in the `LanceDB` store.
///
/// # DTO(store reconciliation identity consumed by Phase 7 stale-row detection)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct IndexedDocumentStem {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
}

/// Query all distinct `(package, document_stem)` pairs currently stored in the `LanceDB` table.
///
/// Returns an empty list when the store directory has not been created yet.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the table cannot be queried.
pub async fn query_all_indexed_document_stems(
    store_dir: &Path,
) -> Result<Vec<IndexedDocumentStem>, LanceDbStoreError> {
    use arrow_array::{Array, cast::AsArray};
    use futures::TryStreamExt;

    if !store_dir.exists() {
        return Ok(Vec::new());
    }
    let table = match open_primary_table(store_dir).await {
        Ok(table) => table,
        Err(LanceDbStoreError::OpenPrimaryTable { .. }) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let stream = table
        .query()
        .select(lancedb::query::Select::columns(&["package", "document_stem"]))
        .execute()
        .await
        .map_err(|error| LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        })?;

    let batches: Vec<arrow_array::RecordBatch> =
        stream.try_collect().await.map_err(|error| LanceDbStoreError::OpenPrimaryTable {
            table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
            message: error.to_string(),
        })?;

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for batch in &batches {
        let package_col =
            batch.column_by_name("package").ok_or_else(|| LanceDbStoreError::OpenPrimaryTable {
                table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                message: "package column missing in query result".to_owned(),
            })?;
        let stem_col = batch.column_by_name("document_stem").ok_or_else(|| {
            LanceDbStoreError::OpenPrimaryTable {
                table: LANCEDB_PRIMARY_CHUNK_TABLE.to_owned(),
                message: "document_stem column missing in query result".to_owned(),
            }
        })?;

        let packages = package_col.as_string::<i32>();
        let stems = stem_col.as_string::<i32>();

        for row in 0..batch.num_rows() {
            let package =
                if packages.is_null(row) { None } else { Some(packages.value(row).to_owned()) };
            let stem = stems.value(row).to_owned();
            let key = (package.clone(), stem.clone());
            if seen.insert(key) {
                result.push(IndexedDocumentStem { package, document_stem: stem });
            }
        }
    }

    Ok(result)
}

/// Delete all rows for one governed document directly from the store, scoped to
/// `(package, document_stem)`.
///
/// Returns 0 when the store directory has not been created yet.
///
/// # Errors
/// Returns [`LanceDbStoreError`] when the delete operation fails.
pub async fn delete_indexed_document(
    store_dir: &Path,
    package: Option<&str>,
    document_stem: &str,
) -> Result<u64, LanceDbStoreError> {
    if !store_dir.exists() {
        return Ok(0);
    }
    let table = match open_primary_table(store_dir).await {
        Ok(table) => table,
        Err(LanceDbStoreError::OpenPrimaryTable { .. }) => return Ok(0),
        Err(error) => return Err(error),
    };
    delete_document_rows(&table, package, document_stem).await
}

#[cfg(test)]
#[path = "query_test.rs"]
mod tests;
