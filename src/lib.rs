#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stderr,
    clippy::print_stdout
)]

//sea_orm does not support setting the table name dynamically
pub const TABLE_NAME: &str = "sessions";

#[cfg(feature = "db_pool")]
mod db_pool;
#[cfg(feature = "db_pool")]
mod entities;

#[cfg(feature = "migration")]
pub mod migration;

#[cfg(feature = "memory_pool")]
pub mod memory_pool;

#[cfg(feature = "db_pool")]
pub use db_pool::*;

#[cfg(feature = "memory_pool")]
pub use memory_pool::*;
