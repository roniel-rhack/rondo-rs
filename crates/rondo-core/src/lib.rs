//! rondo-core: domain types and read-only SQLite store.

pub mod config;
pub mod domain;
pub mod error;
pub mod export;
pub mod i18n;
pub mod recurrence;
pub mod store;
pub mod telemetry;

pub use error::{Error, Result};
