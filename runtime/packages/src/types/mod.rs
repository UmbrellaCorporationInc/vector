//! Package manifest configuration types and models.

pub mod manifest;

pub use manifest::{ManifestError, PackageEntry, PackageManifest, load_manifest, save_manifest};
