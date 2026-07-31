use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    path::Path,
    sync::Arc,
};

use futures::{TryStreamExt, pin_mut};
use loadsmith_core::{PackageId, PackageRef, Version};
use loadsmith_registry::ResolvedVersion;
use parking_lot::{Mutex, RawMutex, lock_api::MutexGuard};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{Error, PackageIdExt, Result};

/// An in-memory package index backed by a thunderstore client.
///
/// Stores package data in a [`HashMap`] and supports serialisation to/from
/// disk for caching.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::in_memory::InMemoryIndex;
/// use thunderstore::Client;
///
/// let client = Client::new();
/// let index = InMemoryIndex::new(client);
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryIndex {
    client: thunderstore::Client,
    state: Arc<Mutex<State>>,
}

/// The serialisable state of an in-memory index.
///
/// Maps community names to their indexed packages.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::in_memory::State;
///
/// let state = State::default();
/// ```
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State(HashMap<String, Community>);

/// A community package collection within an in-memory index.
///
/// Tracks whether the index is complete and stores packages by their
/// [`PackageId`].
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::in_memory::Community;
///
/// let community = Community::default();
/// ```
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Community {
    complete: bool,
    packages: HashMap<PackageId, thunderstore::models::PackageV1>,
}

impl InMemoryIndex {
    /// Creates a new empty in-memory index.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// ```
    pub fn new(client: thunderstore::Client) -> Self {
        Self::new_with_state(client, State::default())
    }

    /// Loads a previously saved index from disk, or creates a new one if the
    /// file does not exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::load(Client::new(), "./index.yaml").unwrap();
    /// ```
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

    /// Serialises the index state to a JSON file on disk.
    ///
    /// The `filter` closure controls which packages are included.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// index.save("./index.json", |_| true).unwrap();
    /// ```
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

    /// Acquires the inner lock on the index state.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// let _state = index.lock();
    /// ```
    pub fn lock(&self) -> MutexGuard<'_, RawMutex, State> {
        self.state.lock()
    }

    /// Fetches and stores the package index for a thunderstore community.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     index.update("lethal-company").await.unwrap();
    /// });
    /// ```
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

    /// Returns version information for a package, if found.
    ///
    /// Returns `None` when the index is complete and the package is not
    /// present. Returns an `IndexNotComplete` error when the index has not
    /// been fully populated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_core::PackageId;
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let versions = index.version_info(&PackageId::new("Author-Pkg")).await.unwrap();
    /// });
    /// ```
    pub async fn version_info(
        &self,
        id: &PackageId,
    ) -> Result<Option<Vec<loadsmith_registry::VersionInfo>>> {
        let lock = self.lock();

        match lock.get(id) {
            Ok(Some(package)) => {
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
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Resolves a package reference to a specific version's download URL,
    /// size, and dependencies.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_core::{PackageId, PackageRef, Version};
    /// use loadsmith_thunderstore::in_memory::InMemoryIndex;
    /// use thunderstore::Client;
    ///
    /// let index = InMemoryIndex::new(Client::new());
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let resolved = index
    ///         .resolve(&PackageRef::new(
    ///             PackageId::new("Author-Pkg"),
    ///             Version::new(1, 0, 0),
    ///         ))
    ///         .await
    ///         .unwrap();
    /// });
    /// ```
    pub async fn resolve(&self, ref_: &PackageRef) -> Result<Option<ResolvedVersion>> {
        let lock = self.lock();

        match lock.get(ref_.id()) {
            Ok(Some(package)) => {
                let Some(version) = package
                    .versions
                    .iter()
                    .find(|v| Version::from(v.number.clone()) == *ref_.version())
                else {
                    return Ok(None);
                };

                let is_modpack = package.categories.contains(super::MODPACK_CATEGORY);
                let deps = super::dependencies_from_idents(&version.dependencies, is_modpack);

                Ok(Some(ResolvedVersion {
                    url: version.download_url.clone().into(),
                    size: Some(version.file_size),
                    checksum: None,
                    deps,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl State {
    /// Loads state from a JSON file on disk.
    fn load(path: &Path) -> Result<Self> {
        debug!(path = %path.display(), "loading index from disk");

        let file = std::fs::File::open(path).map(BufReader::new)?;
        let state = serde_yaml_ng::from_reader(file)?;

        Ok(state)
    }

    /// Serialises the state to a JSON file.
    fn save<F>(&self, path: &Path, _filter: F) -> Result<()>
    where
        F: FnMut(&PackageId) -> bool,
    {
        debug!(path = %path.display(), "writing index to disk");

        let file = std::fs::File::create(path).map(BufWriter::new)?;
        serde_json::to_writer(file, self)?;

        Ok(())
    }

    /// Looks up a package across all indexed communities.
    ///
    /// Returns `IndexNotComplete` if any community index has not finished
    /// fetching.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::in_memory::State;
    /// use loadsmith_core::PackageId;
    ///
    /// let state = State::default();
    /// let result = state.get(&PackageId::new("Unknown-Pkg"));
    /// assert!(result.is_err()); // index is not complete
    /// ```
    pub fn get<'a>(
        &'a self,
        id: &PackageId,
    ) -> Result<Option<&'a thunderstore::models::PackageV1>> {
        let complete = !self.0.is_empty() && self.0.values().all(|community| community.complete);

        match self.0.values().find_map(|community| community.get(id)) {
            Some(package) => Ok(Some(package)),
            None if !complete => Err(Error::IndexNotComplete),
            None => Ok(None),
        }
    }

    /// Returns a mutable reference to the community entry, creating it if
    /// it does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::in_memory::State;
    ///
    /// let mut state = State::default();
    /// let community = state.community_mut("rounds");
    /// ```
    pub fn community_mut(&mut self, community: impl Into<String>) -> &mut Community {
        self.0.entry(community.into()).or_default()
    }
}

impl Community {
    /// Inserts a batch of packages into this community's index.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::in_memory::Community;
    ///
    /// let mut community = Community::default();
    /// // PackageV1 batches are typically obtained from the thunderstore API
    /// ```
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

    /// Looks up a package by its [`PackageId`].
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::in_memory::Community;
    /// use loadsmith_core::PackageId;
    ///
    /// let community = Community::default();
    /// assert!(community.get(&PackageId::new("Unknown-Pkg")).is_none());
    /// ```
    pub fn get(&self, id: &PackageId) -> Option<&thunderstore::models::PackageV1> {
        self.packages.get(id)
    }
}
