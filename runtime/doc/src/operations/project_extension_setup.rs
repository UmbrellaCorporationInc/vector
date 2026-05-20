//! Plugin operation for documentation-owned extension setup of an already-created project.

use runtime_core::{
    RuntimeResult, declare_plugin_operations, operation::FlowOperation, plugin::PluginSender,
};
use runtime_io::path::IoPath;

use crate::operations::create_document_rule::{
    CreateDocumentRuleOp, CreateDocumentRuleOutput, documentation_rule_input,
};

/// Input for the `project_extension_setup` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ProjectExtensionSetupInput {
    /// The root directory of the already-created project.
    pub root_dir: IoPath,
}

impl ProjectExtensionSetupInput {
    /// Constructs a new input for the project extension setup operation.
    #[must_use]
    pub const fn new(root_dir: IoPath) -> Self {
        Self { root_dir }
    }
}

/// Output for the `project_extension_setup` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ProjectExtensionSetupOutput {
    /// The path where the documentation rule was written.
    pub documentation_rule_path: IoPath,
}

async fn project_extension_setup(
    input: ProjectExtensionSetupInput,
    output: &mut impl PluginSender<ProjectExtensionSetupOutput>,
) -> RuntimeResult<()> {
    let rule_input = documentation_rule_input(input.root_dir);
    let rule_output_path = rule_input.output_path.clone();

    let mut rule_output: Option<CreateDocumentRuleOutput> = None;
    let mut collecting_sender = CollectingSender { inner: &mut rule_output };

    CreateDocumentRuleOp.run(rule_input, &mut collecting_sender).await?;

    let _ = rule_output.ok_or_else(|| {
        runtime_core::RuntimeError::operation("documentation rule creation produced no output")
    })?;

    output.send(ProjectExtensionSetupOutput { documentation_rule_path: rule_output_path }).await?;

    Ok(())
}

struct CollectingSender<'a> {
    inner: &'a mut Option<CreateDocumentRuleOutput>,
}

impl runtime_core::channel::Sender<CreateDocumentRuleOutput> for CollectingSender<'_> {
    async fn send(&mut self, value: CreateDocumentRuleOutput) -> RuntimeResult<()> {
        *self.inner = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<CreateDocumentRuleOutput> for CollectingSender<'_> {
    fn is_cancelled(&self) -> bool {
        false
    }
}

declare_plugin_operations! {
    /// Operation that performs documentation-owned extension setup for an already-created project.
    ProjectExtensionSetupOp => project_extension_setup(ProjectExtensionSetupInput, ProjectExtensionSetupOutput)
}

impl Default for ProjectExtensionSetupOp {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
#[path = "project_extension_setup_test.rs"]
mod tests;
