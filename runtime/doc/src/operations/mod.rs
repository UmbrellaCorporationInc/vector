//! Plugin operations for documentation governance.

pub mod bootstrap_doc;
pub mod bootstrap_doc_type;
pub mod create_doc;
pub mod create_doc_type;
pub mod create_document_rule;
pub mod find_doc;
pub mod get_doc_types_tags;
pub mod patch_doc;
pub mod project_extension_setup;
pub(crate) mod support;
pub mod validate;
mod validate_fix;

pub use bootstrap_doc::*;
pub use bootstrap_doc_type::*;
pub use create_doc::*;
pub use create_doc_type::*;
pub use create_document_rule::*;
pub use find_doc::*;
pub use get_doc_types_tags::*;
pub use patch_doc::*;
pub use project_extension_setup::*;
pub use validate::*;
