//! Domain-level channel aliases for `runtime-io`.
//!
//! [`Writer<T>`] and [`Reader<T>`] are named sub-traits over the transport-neutral
//! [`Sender<T>`][runtime_core::channel::Sender] and
//! [`Receiver<T>`][runtime_core::channel::Receiver] contracts defined in `runtime-core`.
//!
//! Both traits carry blanket implementations so every existing concrete
//! `Sender<T>` or `Receiver<T>` automatically satisfies the corresponding alias
//! without any changes to the implementing type.

use runtime_core::channel::{Receiver, Sender};

/// Named sub-trait of [`Sender<T>`] for I/O-oriented output boundaries.
///
/// Every type that implements `Sender<T>` automatically implements `Writer<T>`
/// via the blanket implementation below.
pub trait Writer<T>: Sender<T> {}

impl<T, S: Sender<T>> Writer<T> for S {}

/// Named sub-trait of [`Receiver<T>`] for I/O-oriented input boundaries.
///
/// Every type that implements `Receiver<T>` automatically implements `Reader<T>`
/// via the blanket implementation below.
pub trait Reader<T>: Receiver<T> {}

impl<T, R: Receiver<T>> Reader<T> for R {}

#[cfg(test)]
#[path = "alias_test.rs"]
mod tests;
