//! Command implementation to add a new package to the manifest.

use runtime_core::operation::FlowOperation;
use runtime_io::path::IoPath;
use runtime_packages::operations::add_package::{AddPackageInput, AddPackageOp, AddPackageOutput};

#[derive(Default)]
struct CollectAddSender {
    output: Option<AddPackageOutput>,
}

impl runtime_core::channel::Sender<AddPackageOutput> for CollectAddSender {
    async fn send(&mut self, value: AddPackageOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<AddPackageOutput> for CollectAddSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Runs the package add process.
///
/// # Errors
///
/// Returns an error message if the package cannot be added to the manifest.
pub async fn run(
    root_dir: &std::path::Path,
    name: String,
    r#type: String,
    url: String,
    tag: Option<String>,
) -> Result<(), String> {
    let io_root = IoPath::new(root_dir);
    let input = AddPackageInput::new(io_root, name, r#type, url, tag);
    let mut sender = CollectAddSender::default();

    AddPackageOp::new()
        .run(input, &mut sender)
        .await
        .map_err(|e| format!("add-package failed: {e}"))?;

    Ok(())
}

#[cfg(test)]
#[path = "package_add_test.rs"]
mod tests;
