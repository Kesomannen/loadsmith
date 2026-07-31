//! Registry abstraction and built-in sources for the loadsmith mod-manager
//! library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! ```rust
//! use loadsmith_registry::{RegistrySet, OfflineRegistry, Package, PackageVersion};
//! use loadsmith_core::{PackageId, Version, FileUrl};
//! use std::collections::HashMap;
//!
//! // Register an offline (pre-defined) source alongside other registries.
//! let mut set = RegistrySet::new();
//!
//! let pkg = Package::new(
//!     PackageId::new("author-name"),
//!     vec![PackageVersion::new(
//!         Version::new(1, 0, 0),
//!         FileUrl::try_from_url("https://example.com/pkg.zip").unwrap(),
//!     )],
//! );
//! let mut packages = HashMap::new();
//! packages.insert(PackageId::new("author-name"), pkg);
//! set.add("offline", OfflineRegistry::new(packages));
//!
//! let registry = set.get("offline").expect("offline registry should exist");
//! assert!(format!("{registry:?}").contains("OfflineRegistry"));
//! ```

use std::{collections::HashMap, fmt::Debug, pin::Pin};

use loadsmith_core::{Checksum, Dependency, FileUrl, PackageId, PackageRef, Version};
use serde::de::DeserializeOwned;

mod error;

mod registries;

pub use error::{Error, Result};
pub use registries::local::{self, LocalRegistry};
pub use registries::offline::{self, OfflineRegistry};

/// A single available version reported by a registry.
///
/// Returned by [`Registry::version_info`] to list which versions of a package
/// the registry can provide.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// The concrete version number (e.g. `1.0.0`).
    pub version: Version,
}

/// The fully resolved metadata for a specific package version.
///
/// Produced by [`Registry::resolve`] and contains all information needed to
/// download or verify a package: the download URL, optional size and checksum,
/// and the dependency list.
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    /// URL (or local file path) where the package archive can be obtained.
    pub url: FileUrl,
    /// Uncompressed size of the package archive in bytes, if known.
    pub size: Option<u64>,
    /// Cryptographic checksum of the package archive, if the registry provides one.
    pub checksum: Option<Checksum>,
    /// Direct dependencies declared by this package version.
    pub deps: Vec<Dependency>,
}

/// A source that can list available versions and resolve package references.
///
/// Every [`Registry`] implementation must be [`Send`] + [`Sync`] so it can be
/// shared across asynchronous tasks. The trait provides three operations:
///
/// * [`version_info`](Registry::version_info) — list all known versions of a package.
/// * [`resolve`](Registry::resolve) — turn a [`PackageRef`] into a [`ResolvedVersion`] with a download URL, checksum, and dependencies.
/// * [`revalidate_checksum`](Registry::revalidate_checksum) — optionally re-compute a checksum from the original source (default: no-op).
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::{OfflineRegistry, Package, PackageVersion, Registry};
/// use loadsmith_core::{PackageId, Version, FileUrl, PackageRef};
/// use std::collections::HashMap;
///
/// # #[tokio::main]
/// # async fn main() {
/// let package = Package::new(
///     PackageId::new("author-name"),
///     vec![PackageVersion::new(
///         Version::new(1, 0, 0),
///         FileUrl::try_from_url("https://example.com/pkg.zip").unwrap(),
///     )],
/// );
/// let mut packages = HashMap::new();
/// packages.insert(PackageId::new("author-name"), package);
/// let registry = OfflineRegistry::new(packages);
///
/// let ref_ = PackageRef::new("author-name", Version::new(1, 0, 0));
/// let resolved = registry.resolve(&ref_, None).await.unwrap();
/// assert!(resolved.url.to_string().contains("example.com"));
/// # }
/// ```
pub trait Registry: Debug + Send + Sync {
    /// List all available versions of the package identified by `id`.
    ///
    /// Some registries require additional metadata (e.g. a local filesystem path)
    /// which must be supplied via the `metadata` parameter as a JSON value.
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VersionInfo>>> + 'a>>;

    /// Resolve a package reference into concrete download information.
    ///
    /// Returns a [`ResolvedVersion`] containing the download URL, file size,
    /// checksum, and dependency list for the exact version requested by `ref_`.
    fn resolve<'a>(
        &'a self,
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion>> + 'a>>;

    /// Re-compute and return the checksum for the package identified by `ref_`,
    /// or return `None` if the registry does not support checksum revalidation.
    ///
    /// The default implementation returns `Ok(None)`.
    fn revalidate_checksum<'a>(
        &'a self,
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Result<Option<Checksum>> {
        let _ = (ref_, metadata);
        Ok(None)
    }
}

/// A named collection of [`Registry`] implementations.
///
/// Each registry is stored under a string identifier (e.g. `"thunderstore"`,
/// `"local"`) and can be looked up by name at runtime.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::{RegistrySet, LocalRegistry};
///
/// let mut set = RegistrySet::new();
/// assert!(set.get("local").is_none());
///
/// // Registries can be added and retrieved by name.
/// set.add("local", LocalRegistry::new());
/// assert!(set.get("local").is_some());
/// ```
#[derive(Debug)]
pub struct RegistrySet {
    registries: HashMap<String, Box<dyn Registry>>,
}

impl Default for RegistrySet {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistrySet {
    /// Create an empty set of registries.
    pub fn new() -> Self {
        Self {
            registries: HashMap::new(),
        }
    }

    /// Register a registry under the given identifier.
    ///
    /// If a registry with the same `id` already exists, it is replaced.
    pub fn add<R: Registry + 'static>(&mut self, id: impl Into<String>, registry: R) {
        self.registries.insert(id.into(), Box::new(registry));
    }

    /// Look up a registry by its identifier.
    ///
    /// Returns `None` if no registry has been registered under that name.
    pub fn get(&self, id: &str) -> Option<&dyn Registry> {
        self.registries.get(id).map(|r| r.as_ref())
    }
}

/// Deserialize `metadata` from an optional JSON value, or return `T::default()`
/// when `metadata` is `None`.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::read_metadata_or_default;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Default)]
/// struct Config {
///     name: String,
///     threshold: f64,
/// }
///
/// // When metadata is provided, it is deserialized.
/// let json = serde_json::json!({"name": "test", "threshold": 0.8});
/// let cfg: Config = read_metadata_or_default(Some(&json)).unwrap();
/// assert_eq!(cfg.name, "test");
///
/// // When metadata is None, the default is returned.
/// let cfg: Config = read_metadata_or_default(None).unwrap();
/// assert_eq!(cfg.name, "");
/// assert_eq!(cfg.threshold, 0.0);
/// ```
pub fn read_metadata_or_default<T: DeserializeOwned + Default>(
    metadata: Option<&serde_json::Value>,
) -> Result<T> {
    match metadata {
        Some(metadata) => read_metadata_some(metadata),
        None => Ok(T::default()),
    }
}

/// Deserialize `metadata` from an optional JSON value.
///
/// Returns [`Error::MissingMetadata`] when `metadata` is `None`.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::read_metadata;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Config {
///     name: String,
/// }
///
/// let json = serde_json::json!({"name": "hello"});
/// let cfg: Config = read_metadata(Some(&json)).unwrap();
/// assert_eq!(cfg.name, "hello");
///
/// let err = read_metadata::<Config>(None);
/// assert!(err.is_err());
/// ```
pub fn read_metadata<T: DeserializeOwned>(metadata: Option<&serde_json::Value>) -> Result<T> {
    match metadata {
        Some(metadata) => read_metadata_some(metadata),
        None => Err(Error::MissingMetadata),
    }
}

fn read_metadata_some<T: DeserializeOwned>(metadata: &serde_json::Value) -> Result<T> {
    serde_json::from_value(metadata.clone()).map_err(Error::InvalidMetadata)
}
