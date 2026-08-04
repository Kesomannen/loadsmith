//! Extract, install and uninstall logic for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the [`loadsmith`](https://crates.io/crates/loadsmith)
//! facade crate instead of using this crate directly.

mod error;
mod ops;
pub mod rule;
mod zip;

pub use error::{Error, Result};
pub use ops::{
    extract::extract,
    install::{ConflictStrategy, install},
    uninstall::uninstall,
};
