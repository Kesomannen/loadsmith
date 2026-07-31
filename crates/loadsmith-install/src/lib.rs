//! Install, remove, and list mod files for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! ```rust
//! use loadsmith_install::ConflictStrategy;
//! use loadsmith_install::install;
//! use loadsmith_install::uninstall;
//! use loadsmith_install::extract;
//! use loadsmith_install::GlobRule;
//! use loadsmith_install::RouteRule;
//! use loadsmith_install::InstallRule;
//! use loadsmith_install::InstallRuleset;
//!
//! // Each public type and function is available at the crate root.
//! let strategy = ConflictStrategy::Overwrite;
//! assert_eq!(format!("{strategy:?}"), "Overwrite");
//! ```

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
