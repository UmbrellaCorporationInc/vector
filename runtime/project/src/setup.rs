//! Plugin operation for global project setup composition.

use runtime_core::{
    RuntimeResult, cancel::CancelableSender, channel::Sender, declare_plugin_operations,
    operation::FlowOperation, plugin::PluginSender,
};
use runtime_doc::operations::project_extension_setup::{
    ProjectExtensionSetupInput, ProjectExtensionSetupOp, ProjectExtensionSetupOutput,
};
use runtime_io::path::IoPath;

use crate::operation::{CreateProjectInput, CreateProjectOp, CreateProjectOutput};

/// Input for the global project setup operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ProjectSetupInput {
    /// The target root directory where the project will be created.
    pub target_dir: IoPath,
    /// The human-readable name of the project.
    pub project_name: String,
    /// Whether to force overwrite existing files.
    pub force: bool,
}

/// Output for the global project setup operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ProjectSetupOutput {
    /// Output from the project creation step.
    pub project: CreateProjectOutput,
    /// Output from the documentation extension setup step.
    pub extension: ProjectExtensionSetupOutput,
}

async fn project_setup(
    input: ProjectSetupInput,
    output: &mut impl PluginSender<ProjectSetupOutput>,
) -> RuntimeResult<()> {
    let target_dir = input.target_dir.clone();

    let create_input = CreateProjectInput {
        target_dir: target_dir.clone(),
        project_name: input.project_name,
        force: input.force,
    };

    let mut project_result: Option<CreateProjectOutput> = None;
    let mut create_sender = CollectCreateSender { inner: &mut project_result };
    CreateProjectOp.run(create_input, &mut create_sender).await?;
    let project_output = project_result.ok_or_else(|| {
        runtime_core::RuntimeError::operation("project creation produced no output")
    })?;

    let extension_input = ProjectExtensionSetupInput::new(target_dir);
    let mut extension_result: Option<ProjectExtensionSetupOutput> = None;
    let mut ext_sender = CollectExtSender { inner: &mut extension_result };
    ProjectExtensionSetupOp::default().run(extension_input, &mut ext_sender).await?;
    let extension_output = extension_result.ok_or_else(|| {
        runtime_core::RuntimeError::operation("project extension setup produced no output")
    })?;

    output
        .send(ProjectSetupOutput { project: project_output, extension: extension_output })
        .await?;

    Ok(())
}

struct CollectCreateSender<'a> {
    inner: &'a mut Option<CreateProjectOutput>,
}

impl Sender<CreateProjectOutput> for CollectCreateSender<'_> {
    async fn send(&mut self, value: CreateProjectOutput) -> RuntimeResult<()> {
        *self.inner = Some(value);
        Ok(())
    }
}

impl CancelableSender<CreateProjectOutput> for CollectCreateSender<'_> {
    fn is_cancelled(&self) -> bool {
        false
    }
}

struct CollectExtSender<'a> {
    inner: &'a mut Option<ProjectExtensionSetupOutput>,
}

impl Sender<ProjectExtensionSetupOutput> for CollectExtSender<'_> {
    async fn send(&mut self, value: ProjectExtensionSetupOutput) -> RuntimeResult<()> {
        *self.inner = Some(value);
        Ok(())
    }
}

impl CancelableSender<ProjectExtensionSetupOutput> for CollectExtSender<'_> {
    fn is_cancelled(&self) -> bool {
        false
    }
}

declare_plugin_operations! {
    /// Operation that orchestrates the complete project setup: bootstrap and documentation extension.
    ProjectSetupOp => project_setup(ProjectSetupInput, ProjectSetupOutput)
}

impl ProjectSetupInput {
    /// Construct a `ProjectSetupInput` with explicit fields.
    #[must_use]
    pub const fn new(target_dir: IoPath, project_name: String, force: bool) -> Self {
        Self { target_dir, project_name, force }
    }
}

impl ProjectSetupOp {
    /// Construct a new `ProjectSetupOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for ProjectSetupOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "setup_test.rs"]
mod tests;
