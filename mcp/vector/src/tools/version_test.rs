#![allow(clippy::expect_used)]

use rmcp::handler::server::wrapper::Parameters;

use crate::release::version::workspace_version;

/// Verifies that `VersionTools::new` and `Default` both construct a usable
/// MCP adapter instance for the version capability group.
#[test]
fn version_tools_constructors_produce_usable_adapter() {
    let _from_new = super::VersionTools::new();
    let _from_default = super::VersionTools::default();
}

/// Verifies that the MCP-facing `get_version` tool returns the canonical
/// workspace version string from the shared release source of truth.
#[tokio::test]
async fn get_version_tool_returns_workspace_version() {
    let tools = super::VersionTools::new();
    let result = tools
        .get_version(Parameters(super::GetVersionParams::default()))
        .await
        .expect("tool execution must succeed");

    assert_eq!(
        result,
        workspace_version(),
        "get_version must return the canonical workspace version string"
    );
}
