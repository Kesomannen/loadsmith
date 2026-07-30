//! Manifest and lockfile format, resolver, and profile state for the
//! loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

mod diff;
mod download;
mod error;
mod lockfile;
mod resolve;
mod state;
mod store;

pub use diff::{Diff, Diffable};
pub use download::download_and_extract;
pub use error::{Error, Result};
pub use lockfile::{LockedPackage, Lockfile};
pub use resolve::resolve;
pub use state::{ProfileState, ProfileStateData};
pub use store::{PackageStore, PackageStoreEntry};
