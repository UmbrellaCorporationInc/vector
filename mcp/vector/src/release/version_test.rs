use super::workspace_version;

/// Verifies that `workspace_version` returns a non-empty string.
#[test]
fn workspace_version_returns_non_empty_string() {
    assert!(
        !workspace_version().is_empty(),
        "workspace_version must return a non-empty version string"
    );
}

/// Verifies that `workspace_version` returns a string with at least two dot-separated components.
#[test]
fn workspace_version_has_semver_shape() {
    let version = workspace_version();
    let component_count = version.split('.').count();
    assert!(
        component_count >= 2,
        "workspace version must have at least major.minor components; got: {version}"
    );
}

/// Verifies that all dot-separated components of `workspace_version` are numeric.
#[test]
fn workspace_version_components_are_numeric() {
    let version = workspace_version();
    for component in version.split('.') {
        assert!(
            component.chars().all(|c| c.is_ascii_digit()),
            "each version component must be numeric; component `{component}` in `{version}` is not"
        );
    }
}
