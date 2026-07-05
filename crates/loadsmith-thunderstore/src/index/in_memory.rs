use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    path::Path,
    sync::Arc,
};

use futures::{TryStreamExt, pin_mut};
use loadsmith_core::{PackageId, Version};
use loadsmith_registry::ResolvedVersion;
use parking_lot::{Mutex, RawMutex, lock_api::MutexGuard};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{Error, PackageIdExt, Result};

#[derive(Debug, Clone)]
pub struct InMemoryIndex {
    client: thunderstore::Client,
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State(HashMap<String, Community>);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Community {
    complete: bool,
    packages: HashMap<PackageId, thunderstore::models::PackageV1>,
}

#[derive(Debug)]
pub enum GetPackage<'a> {
    Found(&'a thunderstore::models::PackageV1),
    NotFound,
    NotComplete,
}

impl InMemoryIndex {
    pub fn new(client: thunderstore::Client) -> Self {
        Self::new_with_state(client, State::default())
    }

    pub fn load(client: thunderstore::Client, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let state = if path.exists() {
            State::load(path)?
        } else {
            debug!(path = %path.display(), "index file does not exist, creating new");
            State::default()
        };
        Ok(Self::new_with_state(client, state))
    }

    pub fn save<F>(&self, path: impl AsRef<Path>, filter: F) -> Result<()>
    where
        F: FnMut(&PackageId) -> bool,
    {
        self.lock().save(path.as_ref(), filter)
    }

    fn new_with_state(client: thunderstore::Client, state: State) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, RawMutex, State> {
        self.state.lock()
    }

    pub async fn update(&self, community: impl Into<String>) -> Result<()> {
        let community = community.into();

        debug!(community, "updating index");

        let stream = self.client.stream_package_index_v1(&community).await?;
        pin_mut!(stream);

        while let Some(batch) = stream.try_next().await? {
            self.lock().community_mut(&community).extend(batch);
        }

        self.lock().community_mut(community).complete = true;

        Ok(())
    }

    pub async fn version_info(
        &self,
        id: &PackageId,
    ) -> Result<Option<Vec<loadsmith_registry::VersionInfo>>> {
        let lock = self.lock();
        match lock.get(id) {
            GetPackage::Found(package) => {
                let versions = package
                    .versions
                    .iter()
                    .map(|version| {
                        let version = version.number.clone().into();

                        loadsmith_registry::VersionInfo { version }
                    })
                    .collect::<Vec<_>>();

                Ok(Some(versions))
            }
            GetPackage::NotComplete => Err(Error::IndexNotComplete),
            GetPackage::NotFound => Ok(None),
        }
    }

    pub async fn resolve(
        &self,
        id: &PackageId,
        version: &Version,
    ) -> Result<Option<ResolvedVersion>> {
        let lock = self.lock();

        match lock.get(id) {
            GetPackage::Found(package) => {
                let Some(version) = package
                    .versions
                    .iter()
                    .find(|v| Version::from(v.number.clone()) == *version)
                else {
                    return Ok(None);
                };

                let is_modpack = package.categories.contains(super::MODPACK_CATEGORY);
                let deps = super::dependencies_from_idents(&version.dependencies, is_modpack);

                Ok(Some(ResolvedVersion {
                    url: version.download_url.to_string(),
                    size: Some(version.file_size),
                    checksum: None,
                    deps,
                }))
            }
            GetPackage::NotComplete => Err(Error::IndexNotComplete),
            GetPackage::NotFound => Ok(None),
        }
    }
}

impl State {
    fn load(path: &Path) -> Result<Self> {
        debug!(path = %path.display(), "loading index from disk");

        let file = std::fs::File::open(path).map(BufReader::new)?;
        let state = serde_yaml_ng::from_reader(file)?;

        Ok(state)
    }

    fn save<F>(&self, path: &Path, _filter: F) -> Result<()>
    where
        F: FnMut(&PackageId) -> bool,
    {
        debug!(path = %path.display(), "writing index to disk");

        let file = std::fs::File::create(path).map(BufWriter::new)?;
        serde_json::to_writer(file, self)?;

        Ok(())
    }

    pub fn get(&self, id: &PackageId) -> GetPackage<'_> {
        let complete = !self.0.is_empty() && self.0.values().all(|community| community.complete);

        self.0
            .values()
            .find_map(|community| community.get(id))
            .map(GetPackage::Found)
            .unwrap_or(if complete {
                GetPackage::NotFound
            } else {
                GetPackage::NotComplete
            })
    }

    pub fn community_mut(&mut self, community: impl Into<String>) -> &mut Community {
        self.0.entry(community.into()).or_default()
    }
}

impl Community {
    pub fn extend<I>(&mut self, packages: I)
    where
        I: IntoIterator<Item = thunderstore::models::PackageV1>,
    {
        self.packages.extend(
            packages
                .into_iter()
                .map(|package| (PackageId::from_ts_ident(package.ident.clone()), package)),
        );
    }

    pub fn get(&self, id: &PackageId) -> Option<&thunderstore::models::PackageV1> {
        self.packages.get(id)
    }
}
