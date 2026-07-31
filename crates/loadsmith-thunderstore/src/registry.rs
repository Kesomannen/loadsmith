use std::pin::Pin;

use loadsmith_core::{PackageId, PackageRef};
use loadsmith_registry::{Registry, ResolvedVersion, VersionInfo};

use crate::{Error, in_memory::InMemoryIndex, sqlite::SqliteIndex};

/// A registry that wraps a thunderstore package index.
///
/// Supports both in-memory and SQLite-backed index backends and implements
/// the [`Registry`] trait for mod dependency resolution.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::ThunderstoreRegistry;
///
/// let registry = ThunderstoreRegistry::default();
/// ```
#[derive(Debug, Clone)]
pub struct ThunderstoreRegistry {
    _client: thunderstore::Client,
    index: Index,
}

#[derive(Debug, Clone)]
enum Index {
    InMemory(InMemoryIndex),
    Sqlite(SqliteIndex),
}

impl ThunderstoreRegistry {
    fn new(client: thunderstore::Client, index: Index) -> Self {
        Self {
            _client: client,
            index,
        }
    }

    /// Creates a registry backed by an in-memory index.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::{ThunderstoreRegistry, in_memory::InMemoryIndex};
    /// use thunderstore::Client;
    ///
    /// let client = Client::new();
    /// let index = InMemoryIndex::new(client.clone());
    /// let registry = ThunderstoreRegistry::in_memory(client, index);
    /// ```
    pub fn in_memory(client: thunderstore::Client, index: InMemoryIndex) -> Self {
        Self::new(client, Index::InMemory(index))
    }

    /// Creates a registry backed by a SQLite index.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::{ThunderstoreRegistry, sqlite::SqliteIndex};
    /// use thunderstore::Client;
    ///
    /// let client = Client::new();
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(client.clone(), db).unwrap();
    /// let registry = ThunderstoreRegistry::sqlite(client, index);
    /// ```
    pub fn sqlite(client: thunderstore::Client, index: SqliteIndex) -> Self {
        Self::new(client, Index::Sqlite(index))
    }

    /// Fetches the package index for a thunderstore community.
    ///
    /// The community string corresponds to a thunderstore community slug
    /// (e.g. `"rounds"`, `"lethal-company"`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::ThunderstoreRegistry;
    ///
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let registry = ThunderstoreRegistry::default();
    ///     registry.update("rounds").await.unwrap();
    /// });
    /// ```
    pub async fn update(&self, community: impl Into<String>) -> Result<(), Error> {
        match &self.index {
            Index::InMemory(in_memory_index) => in_memory_index.update(community).await,
            Index::Sqlite(sqlite_index) => sqlite_index.update(community.into()).await,
        }
    }
}

impl Default for ThunderstoreRegistry {
    fn default() -> Self {
        let client = thunderstore::Client::default();
        Self::in_memory(client.clone(), InMemoryIndex::new(client))
    }
}

impl Registry for ThunderstoreRegistry {
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = loadsmith_registry::Result<Vec<VersionInfo>>> + 'a>> {
        Box::pin(async move {
            match &self.index {
                Index::InMemory(in_memory_index) => in_memory_index.version_info(id).await,
                Index::Sqlite(sqlite_index) => sqlite_index.version_info(id),
            }
            .map_err(loadsmith_registry::Error::other)?
            .ok_or(loadsmith_registry::Error::PackageNotFound)
        })
    }

    fn resolve<'a>(
        &'a self,
        ref_: &'a PackageRef,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = loadsmith_registry::Result<ResolvedVersion>> + 'a>> {
        Box::pin(async move {
            match &self.index {
                Index::InMemory(in_memory_index) => in_memory_index.resolve(ref_).await,
                Index::Sqlite(sqlite_index) => sqlite_index.resolve(ref_),
            }
            .map_err(loadsmith_registry::Error::other)?
            .ok_or(loadsmith_registry::Error::VersionNotFound)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn get_package() {
        let registry = ThunderstoreRegistry::default();
        registry.update("rounds").await.unwrap();

        let package = PackageId::new("BepInEx-BepInExPack_ROUNDS");

        let versions = registry.version_info(&package, None).await.unwrap();

        println!("{versions:#?}");
    }
}
