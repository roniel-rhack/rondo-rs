//! rondo-core: domain types and read-only SQLite store.

pub mod domain;
pub mod error;
pub mod store;

pub use error::{Error, Result};
