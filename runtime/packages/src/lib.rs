//! Runtime package management and synchronization for the vector system.

pub mod manifest;

pub use manifest::{ManifestError, PackageEntry, PackageManifest, load_manifest, save_manifest};
