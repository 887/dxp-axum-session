#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

#[cfg(feature = "db_pool")]
mod db_storage;
#[cfg(feature = "db_pool")]
mod entities;

#[cfg(feature = "migration")]
pub mod migration;

#[cfg(feature = "db_pool")]
pub use db_storage::*;
