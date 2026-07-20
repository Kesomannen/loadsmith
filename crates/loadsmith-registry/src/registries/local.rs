use std::{fs::File, io::BufReader, path::Path, pin::Pin};

use camino::Utf8PathBuf;
use loadsmith_core::{Dependency, PackageId, PackageRef, Version, VersionRange};
use serde::Deserialize;
use thunderstore::VersionIdent;

use crate::{Error, Registry, ResolvedVersion, Result, VersionInfo};

#[derive(Debug)]
pub struct LocalRegistry;

impl Default for LocalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalRegistry {
    pub fn new() -> Self {
        Self
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
            let source = Source::read(id, metadata)?;

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

            let source = Source::read_ref(ref_, metadata)?;

            let full_path = source.path.canonicalize_utf8()?;
            let url = format!("file://{}", full_path);

            let size = std::fs::metadata(&source.path)?.len();

            Ok(ResolvedVersion {
                url,
                size: Some(size),
                deps: source.dependencies()?,
                checksum: Some(source.checksum()?),
            })
        })
    }

    fn revalidate_checksum<'a>(
        &'a self,
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Result<Option<loadsmith_core::Checksum>> {
        let metadata = crate::read_metadata(metadata)?;
        let source = Source::read_ref(ref_, metadata)?;

        Ok(Some(source.checksum()?))
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    path: Utf8PathBuf,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "deps_source"
    )]
    dependency_registry: Option<String>,
}

struct Source {
    path: Utf8PathBuf,
    kind: SourceKind,
    dependency_registry: String,
}

enum SourceKind {
    Zip(ThunderstoreManifest),
    Dll,
    Directory,
}

impl Source {
    const DEFAULT_DEPENDENCY_REGISTRY: &'static str = "thunderstore";

    fn read(id: &PackageId, metadata: Metadata) -> Result<Self> {
        if !metadata.path.exists() {
            return Err(Error::FileNotFound(metadata.path));
        }

        let kind = match (metadata.path.is_file(), metadata.path.extension()) {
            (true, Some("zip")) => {
                let manifest = read_zip(&metadata.path)?;

                let ident = format!("{}-{}", manifest.namespace, manifest.name);
                if ident != id.as_str() {
                    return Err(Error::PackageNotFound);
                }

                SourceKind::Zip(manifest)
            }
            (true, Some("dll")) => SourceKind::Dll,
            (false, _) => SourceKind::Directory,
            _ => return Err(Error::InvalidFileType(metadata.path)),
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

    fn read_ref(ref_: &PackageRef, metadata: Metadata) -> Result<Self> {
        let source = Self::read(ref_.id(), metadata)?;

        if source.version() != ref_.version() {
            return Err(Error::VersionNotFound);
        }

        Ok(source)
    }

    fn version(&self) -> &Version {
        match &self.kind {
            SourceKind::Zip(manifest) => &manifest.version_number,
            SourceKind::Dll => todo!(),
            SourceKind::Directory => todo!(),
        }
    }

    fn dependencies(&self) -> Result<Vec<Dependency>> {
        match &self.kind {
            SourceKind::Zip(manifest) => {
                let deps = manifest
                    .dependencies
                    .iter()
                    .map(|ident| {
                        let package_id = PackageId::new(ident.package_id().into_string());

                        Dependency::new(
                            package_id,
                            VersionRange::any(),
                            self.dependency_registry.clone(),
                        )
                    })
                    .collect();

                Ok(deps)
            }
            SourceKind::Dll => todo!(),
            SourceKind::Directory => todo!(),
        }
    }

    fn checksum(&self) -> Result<loadsmith_core::Checksum> {
        match self.kind {
            SourceKind::Zip(_) | SourceKind::Dll => {
                let hash = loadsmith_util::hash_file(&self.path)?;
                Ok(hash.into())
            }
            SourceKind::Directory => todo!(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ThunderstoreManifest {
    namespace: String,
    name: String,
    #[allow(unused)]
    description: String,
    version_number: Version,
    dependencies: Vec<VersionIdent>,
    #[allow(unused)]
    website_url: Option<String>,
}

fn read_zip(path: impl AsRef<Path>) -> Result<ThunderstoreManifest> {
    let file = File::open(path.as_ref()).map(BufReader::new)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut manifest_file = archive.by_name("manifest.json")?;
    let manifest: ThunderstoreManifest = serde_json::from_reader(&mut manifest_file)?;

    Ok(manifest)
}
