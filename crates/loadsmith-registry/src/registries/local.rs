use std::{fs::File, io::BufReader, path::Path, pin::Pin};

use camino::Utf8PathBuf;
use loadsmith_core::{PackageId, PackageRef, Version, VersionReq};
use serde::Deserialize;
use thunderstore::VersionIdent;
use tracing::debug;

use crate::{Dependency, Error, Registry, ResolvedVersion, Result, VersionInfo};

/// A registry that reads package data from the local filesystem.
///
/// `LocalRegistry` expects metadata containing a `"path"` field pointing to a
/// local file or directory. Supported sources are:
///
/// * `.zip` archives containing a Thunderstore-style `manifest.json`
/// * Unpacked directories with a Thunderstore `manifest.json`
/// * `.dll` files (version extracted from metadata or filename)
/// * Other files (version extracted from metadata or filename)
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::LocalRegistry;
///
/// let registry = LocalRegistry::new();
/// assert!(format!("{registry:?}").contains("LocalRegistry"));
/// ```
#[derive(Debug)]
pub struct LocalRegistry;

impl Default for LocalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalRegistry {
    /// Create a new `LocalRegistry`.
    pub fn new() -> Self {
        Self
    }

    pub fn read(&self, metadata: Metadata) -> Result<(Option<PackageId>, VersionInfo)> {
        let source = Source::read(metadata)?;

        Ok((
            source.package_id(),
            VersionInfo {
                version: source.version().clone(),
            },
        ))
    }
}

impl Registry for LocalRegistry {
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VersionInfo>>> + 'a>> {
        Box::pin(async move {
            let metadata = crate::read_metadata(metadata)?;
            let source = Source::read(metadata)?;

            source.check_package_id(id)?;

            Ok(vec![VersionInfo {
                version: source.version().clone(),
            }])
        })
    }

    fn resolve<'a>(
        &'a self,
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion>> + 'a>> {
        Box::pin(async move {
            let metadata = crate::read_metadata(metadata)?;
            let source = Source::read(metadata)?;
            source.check_package_ref(ref_)?;

            let full_path = source.path.canonicalize_utf8()?;

            Ok(ResolvedVersion {
                url: full_path.into(),
                size: source.size()?,
                deps: source.dependencies()?,
                checksum: source.checksum().map(Some)?,
            })
        })
    }

    fn revalidate_checksum<'a>(
        &'a self,
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Result<Option<loadsmith_core::Checksum>> {
        let metadata = crate::read_metadata(metadata)?;
        let source = Source::read(metadata)?;
        source.check_package_ref(ref_)?;

        let checksum = source.checksum()?;

        Ok(Some(checksum))
    }
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    path: Utf8PathBuf,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "deps_source"
    )]
    dependency_registry: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "source_version"
    )]
    source_version: Option<Version>,
}

struct Source {
    path: Utf8PathBuf,
    kind: SourceKind,
    dependency_registry: String,
}

enum SourceKind {
    Zip(ThunderstoreManifest),
    Directory(ThunderstoreManifest),
    Dll(Version),
    Other(Version),
}

impl Metadata {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            dependency_registry: None,
            source_version: None,
        }
    }

    pub fn with_dependency_registry(mut self, registry: impl Into<String>) -> Self {
        self.dependency_registry = Some(registry.into());
        self
    }

    pub fn with_source_version(mut self, version: impl Into<Version>) -> Self {
        self.source_version = Some(version.into());
        self
    }
}

impl Source {
    const DEFAULT_DEPENDENCY_REGISTRY: &'static str = "thunderstore";

    fn read(metadata: Metadata) -> Result<Self> {
        if !metadata.path.exists() {
            return Err(Error::FileNotFound(metadata.path));
        }

        let kind = match (metadata.path.is_file(), metadata.path.extension()) {
            (true, Some("zip")) => {
                let manifest = read_zip_manifest(&metadata.path)?;

                Self::warn_if_source_version_set(&manifest.version_number, &metadata);

                SourceKind::Zip(manifest)
            }
            (true, Some("dll")) => {
                SourceKind::Dll(metadata.source_version.ok_or(Error::LocalVersionMissing)?)
            }
            (false, _) => {
                let manifest_path = metadata.path.join("manifest.json");
                if !manifest_path.exists() {
                    return Err(Error::LocalManifestMissing);
                }

                let manifest_str = std::fs::read_to_string(&manifest_path)?;
                let manifest: ThunderstoreManifest = serde_json::from_str(&manifest_str)?;

                Self::warn_if_source_version_set(&manifest.version_number, &metadata);

                SourceKind::Directory(manifest)
            }
            _ => {
                debug!(path = %metadata.path, "could not determine source kind");

                SourceKind::Other(metadata.source_version.ok_or(Error::LocalVersionMissing)?)
            }
        };

        let dependency_registry = metadata
            .dependency_registry
            .unwrap_or_else(|| Self::DEFAULT_DEPENDENCY_REGISTRY.to_string());

        Ok(Source {
            kind,
            path: metadata.path,
            dependency_registry,
        })
    }

    fn warn_if_source_version_set(package_version: &Version, metadata: &Metadata) {
        if let Some(source_version) = &metadata.source_version {
            debug!(
                path = %metadata.path,
                %source_version,
                %package_version,
                "source_version is set, but is overriden by the local package's version"
            );
        }
    }

    fn check_package_id(&self, id: &PackageId) -> Result<()> {
        match self.package_id() {
            Some(package_id) if package_id == *id => Ok(()),
            Some(_) => Err(Error::PackageNotFound),
            None => Ok(()),
        }
    }

    fn package_id(&self) -> Option<PackageId> {
        match &self.kind {
            SourceKind::Zip(manifest) | SourceKind::Directory(manifest) => {
                let package_id = if let Some(namespace) = &manifest.namespace {
                    format!("{}-{}", namespace, manifest.name)
                } else {
                    manifest.name.clone()
                };

                Some(PackageId::new(package_id))
            }
            SourceKind::Dll(_) | SourceKind::Other(_) => None,
        }
    }

    fn check_package_ref(&self, ref_: &PackageRef) -> Result<()> {
        self.check_package_id(ref_.id())?;

        if self.version() != ref_.version() {
            return Err(Error::VersionNotFound);
        }

        Ok(())
    }

    fn version(&self) -> &Version {
        match &self.kind {
            SourceKind::Zip(manifest) | SourceKind::Directory(manifest) => &manifest.version_number,
            SourceKind::Dll(version) | SourceKind::Other(version) => version,
        }
    }

    fn size(&self) -> Result<Option<u64>> {
        match &self.kind {
            SourceKind::Zip(_) | SourceKind::Dll(_) | SourceKind::Other(_) => {
                let size = std::fs::metadata(&self.path)?.len();
                Ok(Some(size))
            }
            SourceKind::Directory(_) => Ok(None),
        }
    }

    fn dependencies(&self) -> Result<Vec<Dependency>> {
        match &self.kind {
            SourceKind::Zip(manifest) | SourceKind::Directory(manifest) => {
                let deps = manifest
                    .dependencies
                    .iter()
                    .map(|ident| {
                        let package_id = PackageId::new(ident.package_id().into_string());

                        Dependency::new(
                            package_id,
                            VersionReq::STAR,
                            self.dependency_registry.clone(),
                        )
                    })
                    .collect();

                Ok(deps)
            }
            SourceKind::Dll(_) | SourceKind::Other(_) => Ok(Vec::new()),
        }
    }

    fn checksum(&self) -> Result<loadsmith_core::Checksum> {
        loadsmith_core::Checksum::compute_from_path(
            &self.path,
            loadsmith_core::ChecksumAlgorithm::Blake3,
        )
        .map_err(Into::into)
    }
}

#[derive(Debug, Deserialize)]
struct ThunderstoreManifest {
    #[serde(default)]
    namespace: Option<String>,
    name: String,
    #[allow(unused)]
    description: String,
    version_number: Version,
    dependencies: Vec<VersionIdent>,
    #[allow(unused)]
    website_url: Option<String>,
}

fn read_zip_manifest(path: impl AsRef<Path>) -> Result<ThunderstoreManifest> {
    let file = File::open(path.as_ref()).map(BufReader::new)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut manifest_file = archive.by_name("manifest.json").map_err(|err| match err {
        zip::result::ZipError::FileNotFound => Error::LocalManifestMissing,
        other => Error::Zip(other),
    })?;
    let manifest: ThunderstoreManifest = serde_json::from_reader(&mut manifest_file)?;

    Ok(manifest)
}
