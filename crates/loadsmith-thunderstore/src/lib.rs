//! Thunderstore API client, index backends, and r2z format for the loadsmith
//! mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! ```
//! use loadsmith_core::PackageId;
//! use loadsmith_thunderstore::PackageIdExt;
//!
//! let id = PackageId::new("Author-Name");
//! let ident = id.into_ts_ident().unwrap();
//! assert_eq!(ident.to_string(), "Author-Name");
//! ```

mod error;
mod ext;
mod index;
pub mod r2z;
mod registry;
mod schema;

pub use error::{Error, Result};
pub use ext::{PackageIdExt, PackageRefExt};
pub use index::{in_memory, sqlite};
pub use registry::ThunderstoreRegistry;
pub use schema::{distribution_into_platform, r2_config_to_loader};
