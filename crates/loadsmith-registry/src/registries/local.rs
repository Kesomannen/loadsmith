use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    pin::Pin,
};

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

            let source = Source::read(ref_.id(), metadata)?;

            if source.version() != ref_.version() {
                return Err(Error::VersionNotFound);
            }

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

        let source = Source::read(ref_.id(), metadata)?;

        if source.version() != ref_.version() {
            return Err(Error::VersionNotFound);
        }

        Ok(Some(source.checksum()?))
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    path: Utf8PathBuf,
}

struct Source {
    path: Utf8PathBuf,
    kind: SourceKind,
}

enum SourceKind {
    Zip(ThunderstoreManifest),
}

impl Source {
    fn read(id: &PackageId, metadata: Metadata) -> Result<Self> {
        if !metadata.path.is_file() {
            return Err(Error::FileNotFound(metadata.path));
        }

        let kind = match metadata.path.extension() {
            Some("zip") => {
                let manifest = read_zip(&metadata.path)?;

                let ident = format!("{}-{}", manifest.namespace, manifest.name);
                if ident != id.as_str() {
                    return Err(Error::PackageNotFound);
                }

                SourceKind::Zip(manifest)
            }
            _ => return Err(Error::InvalidFileType(metadata.path)),
        };

        Ok(Source {
            kind,
            path: metadata.path,
        })
    }

    fn version(&self) -> &Version {
        match &self.kind {
            SourceKind::Zip(manifest) => &manifest.version_number,
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

                        // TODO: better source handling instead of hardcoding thunderstore
                        // TODO: better version range handling
                        Dependency::new(package_id, VersionRange::any(), "thunderstore".to_string())
                    })
                    .collect();

                Ok(deps)
            }
        }
    }

    fn checksum(&self) -> Result<loadsmith_core::Checksum> {
        let checksum = loadsmith_util::hash_file(&self.path)?.into();
        Ok(checksum)
    }
}

#[derive(Debug, Deserialize)]
struct ThunderstoreManifest {
    namespace: String,
    name: String,
    // description: String,
    version_number: Version,
    dependencies: Vec<VersionIdent>,
    // website_url: Option<String>,
}

fn read_zip(path: impl AsRef<Path>) -> Result<ThunderstoreManifest> {
    let file = File::open(path.as_ref()).map(BufReader::new)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut manifest_str = String::new();
    archive
        .by_name("manifest.json")?
        .read_to_string(&mut manifest_str)?;
    let manifest: ThunderstoreManifest = serde_json::from_str(&manifest_str)?;

    Ok(manifest)
}
