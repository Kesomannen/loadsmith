//! Install, remove, and list mod files for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

mod error;
mod ops;
mod rule;
mod zip;

pub use error::{Error, Result};
pub use ops::{
    extract::extract,
    install::{ConflictStrategy, install},
    uninstall::uninstall,
};
pub use rule::{GlobRule, InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};
