//! Plugin operation for retrieving all tags across document types.

use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::collections::BTreeSet;

use crate::types::load_document_types_config;

/// Input for the `get_doc_types_tags` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct GetDocTypesTagsInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
}

/// Output for the `get_doc_types_tags` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct GetDocTypesTagsOutput {
    /// Comma-separated string of unique, sorted tags.
    pub tags: String,
}

async fn get_doc_types_tags(
    input: GetDocTypesTagsInput,
    output: &mut impl PluginSender<GetDocTypesTagsOutput>,
) -> RuntimeResult<()> {
    let config = load_document_types_config(&input.root_dir).await?;

    let mut all_tags = BTreeSet::new();

    for type_config in config.document_types.values() {
        if let Some(tags) = &type_config.tags {
            for tag in tags {
                all_tags.insert(tag.clone());
            }
        }
    }

    let tags_string = all_tags.into_iter().collect::<Vec<_>>().join(",");

    output.send(GetDocTypesTagsOutput { tags: tags_string }).await?;

    Ok(())
}

declare_plugin_operations! {
    GetDocTypesTagsOp => get_doc_types_tags(GetDocTypesTagsInput, GetDocTypesTagsOutput)
}

#[cfg(test)]
#[path = "get_doc_types_tags_test.rs"]
mod tests;
