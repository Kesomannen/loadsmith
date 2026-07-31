//! Manifest and lockfile format, resolver, and profile state for the
//! loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! Create a lockfile and compute a diff between two revisions:
//!
//! ```rust
//! use loadsmith_core::{FileUrl, PackageRef, Version};
//! use loadsmith_manifest::{Diff, LockedPackage, Lockfile};
//!
//! let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
//!
//! let old = Lockfile::new(vec![
//!     LockedPackage::new(PackageRef::new("Author-A", Version::new(1, 0, 0)), "thunderstore", url.clone()),
//! ]);
//!
//! let new = Lockfile::new(vec![
//!     LockedPackage::new(PackageRef::new("Author-B", Version::new(1, 0, 0)), "thunderstore", url),
//! ]);
//!
//! let diff = old.diff(&new);
//! assert_eq!(diff.added.len(), 1);
//! assert_eq!(diff.removed.len(), 1);
//! assert!(diff.changed.is_empty());
//! ```
//!
//! Resolve dependencies against a registry and produce a lockfile:
//!
//! ```rust,no_run
//! # async fn example() {
//! use loadsmith_core::{Dependency, VersionReq};
//! use loadsmith_manifest::resolve;
//! use loadsmith_registry::RegistrySet;
//!
//! let registries = RegistrySet::new();
//! let deps = vec![Dependency::new("Author-Package", VersionReq::STAR, "thunderstore")];
//! let lockfile = resolve(deps, &registries, None).await.unwrap();
//! # }
//! ```

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
