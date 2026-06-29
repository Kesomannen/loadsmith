use std::{
    fs::File,
    path::{Path, PathBuf},
    pin::Pin,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{Error, Package, PackageVersion, Registry, RegistryId, Result};

pub struct LocalRegistry {}

impl LocalRegistry {
    pub fn new() -> Self {
        Self {}
    }
}

impl Registry for LocalRegistry {
    fn id(&self) -> RegistryId {
        RegistryId("local")
    }

    fn get_package<'a>(
        &'a self,
        id: &'a loadsmith_core::PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<crate::Package>>> + 'a>> {
        Box::pin(async move {
            let metadata: Metadata = metadata.ok_or(Error::MissingMetadata).and_then(|json| {
                serde_json::from_value(json.clone())
                    .map_err(|error| Error::InvalidMetadata { error })
            })?;

            if !metadata.path.is_file() {
                return Err(Error::FileNotFound(metadata.path));
            }

            let checksum = file_checksum(&metadata.path)?;

            // TODO: read zip file and extract version, deps, etc.

            let version = PackageVersion {
                version: todo!(),
                download_url: todo!(),
                checksum: Some(checksum),
                deps: todo!(),
            };

            let package = Package {
                versions: vec![version],
            };

            Ok(Some(package))
        })
    }
}

fn file_checksum(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path.as_ref())?;
    let mut hasher = digest_io::IoWrapper(Sha256::new());
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.0.finalize();

    Ok(format!("{hash:x?}"))
}

#[derive(Debug, Deserialize)]
struct Metadata {
    path: PathBuf,
}
