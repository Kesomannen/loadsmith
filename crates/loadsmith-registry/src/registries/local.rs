use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    pin::Pin,
};

use camino::Utf8PathBuf;
use loadsmith_core::{Dependency, PackageId, Version, VersionRange};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thunderstore::VersionIdent;

use crate::{Error, Registry, ResolvedVersion, Result, VersionInfo};

#[derive(Debug)]
pub struct LocalRegistry;

impl LocalRegistry {
    pub fn new() -> Self {
        Self
    }
}

impl Registry for LocalRegistry {
    fn version_info<'a>(
        &'a self,
        id: &'a loadsmith_core::PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VersionInfo>>> + 'a>> {
        Box::pin(async move {
            let source = Source::read(id, metadata)?;
            match &source.kind {
                SourceKind::Zip(manifest) => {
                    let version_info = VersionInfo {
                        version: manifest.version_number,
                    };

                    Ok(vec![version_info])
                }
            }
        })
    }

    fn resolve<'a>(
        &'a self,
        id: &'a PackageId,
        version: &'a Version,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion>> + 'a>> {
        Box::pin(async move {
            let source = Source::read(id, metadata)?;

            let full_path = source.path.canonicalize_utf8()?;
            let url = format!("file://{}", full_path);

            let size = std::fs::metadata(&source.path)?.len();
            let checksum = source.checksum()?;

            let deps = match source.kind {
                SourceKind::Zip(manifest) => {
                    if &manifest.version_number != version {
                        return Err(Error::VersionNotFound);
                    }

                    manifest
                        .dependencies
                        .into_iter()
                        .map(|ident| {
                            let package_id = PackageId::new(ident.package_id().into_string());

                            // TODO: better source handling instead of random string
                            // TODO: better version range handling
                            Dependency::new(
                                package_id,
                                VersionRange::any(),
                                "thunderstore".to_string(),
                            )
                        })
                        .collect()
                }
            };

            Ok(ResolvedVersion {
                url,
                size: Some(size),
                checksum: Some(checksum),
                deps,
            })
        })
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
    fn read(id: &loadsmith_core::PackageId, metadata: Option<&serde_json::Value>) -> Result<Self> {
        let metadata: Metadata = metadata.ok_or(Error::MissingMetadata).and_then(|json| {
            serde_json::from_value(json.clone()).map_err(|error| Error::InvalidMetadata { error })
        })?;

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

    fn checksum(&self) -> Result<String> {
        let mut file = File::open(&self.path)?;
        let mut hasher = digest_io::IoWrapper(Sha256::new());
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.0.finalize();

        Ok(format!("{hash:x?}"))
    }
}

#[derive(Debug, Deserialize)]
struct ThunderstoreManifest {
    namespace: String,
    name: String,
    description: String,
    version_number: Version,
    dependencies: Vec<VersionIdent>,
    website_url: Option<String>,
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
