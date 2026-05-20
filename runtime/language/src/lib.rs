//! Language-focused runtime operations for governed prompt resolution.
//!
//! This crate owns reusable language operations that should remain transport-agnostic.

pub mod operation;

pub use operation::*;
