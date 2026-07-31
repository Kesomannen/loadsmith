use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use futures::{TryStreamExt, pin_mut};
use loadsmith_core::{PackageId, PackageRef, Version};
use parking_lot::Mutex;
use rusqlite::OptionalExtension;
use thunderstore::VersionIdent;
use tracing::{debug, trace};

use crate::Result;

/// A SQLite-backed package index using the thunderstore API.
///
/// Stores package data in a local SQLite database with WAL journal mode for
/// concurrent reads.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::sqlite::SqliteIndex;
/// use thunderstore::Client;
///
/// let client = Client::new();
/// let db = rusqlite::Connection::open_in_memory().unwrap();
/// let index = SqliteIndex::connect(client, db).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct SqliteIndex {
    client: thunderstore::Client,
    db: Arc<Mutex<rusqlite::Connection>>,
}

/// Metadata about a community's last index update.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::sqlite::CommunityMetadata;
///
/// let meta = CommunityMetadata {
///     community: "rounds".into(),
///     last_updated: None,
/// };
/// assert_eq!(meta.community, "rounds");
/// ```
#[derive(Debug, Clone)]
pub struct CommunityMetadata {
    pub community: String,
    pub last_updated: Option<DateTime<Utc>>,
}

impl SqliteIndex {
    /// Creates a new SQLite index from an existing connection and initialises
    /// the schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// ```
    pub fn connect(client: thunderstore::Client, db: rusqlite::Connection) -> Result<Self> {
        db.execute_batch(include_str!("queries/create_schema.sql"))?;
        Ok(Self {
            client,
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Opens a SQLite database file and initialises the schema.
    ///
    /// Enables WAL journal mode for better concurrent read performance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let index = SqliteIndex::open(Client::new(), "./thunderstore.db").unwrap();
    /// ```
    pub fn open(client: thunderstore::Client, db_path: impl AsRef<Path>) -> Result<Self> {
        let db = rusqlite::Connection::open(db_path)?;
        db.pragma_update(None, "journal_mode", "WAL")?;
        Self::connect(client, db)
    }

    /// Returns metadata about a community, including the last update time.
    ///
    /// Returns `None` if the community has not been indexed yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// let meta = index.community_metadata("rounds").unwrap();
    /// assert!(meta.is_none());
    /// ```
    pub fn community_metadata(
        &self,
        community: impl AsRef<str>,
    ) -> Result<Option<CommunityMetadata>> {
        let db = self.db.lock();

        let metadata = db
            .prepare("select * from community_meta where community = ?")?
            .query_row(rusqlite::params![community.as_ref()], |row| {
                let community: String = row.get(0)?;
                let last_updated: Option<String> = row.get(1)?;
                let last_updated = last_updated
                    .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()
                    .unwrap();

                Ok(CommunityMetadata {
                    community,
                    last_updated,
                })
            })
            .optional()?;

        Ok(metadata)
    }

    fn set_community_metadata(
        &self,
        community: impl AsRef<str>,
        last_updated: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let db = self.db.lock();

        db.execute(
            "insert or replace into community_meta (community, last_updated) values (?1, ?2)",
            rusqlite::params![community.as_ref(), last_updated.map(|dt| dt.to_rfc3339())],
        )?;

        Ok(())
    }

    /// Fetches and stores the package index for a thunderstore community.
    ///
    /// Inserts or replaces all packages for the given community and records
    /// the update timestamp.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     index.update("lethal-company").await.unwrap();
    /// });
    /// ```
    pub async fn update(&self, community: impl AsRef<str>) -> Result<()> {
        let community = community.as_ref();

        debug!(community, "fetching package index from thunderstore");

        let mut start = Instant::now();
        let mut time_waiting = Duration::ZERO;

        let stream = self.client.stream_package_index_v1(community).await?;
        pin_mut!(stream);

        time_waiting += start.elapsed();
        start = Instant::now();

        while let Some(batch) = stream.try_next().await? {
            trace!(len = batch.len(), waited = ?start.elapsed(), "received package batch");

            time_waiting += start.elapsed();
            start = Instant::now();

            let mut db = self.db.lock();
            let tx = db.transaction()?;

            for package in batch {
                tx.execute(
                    "insert or replace into packages (community, package_id, package) values (?1, ?2, ?3)",
                    rusqlite::params![community, package.ident.as_str(), serde_json::to_string(&package)?],
                )?;
            }

            tx.commit()?;

            trace!(
                duration = ?start.elapsed(),
                "inserted package batch into database"
            );

            start = Instant::now();
        }

        self.set_community_metadata(community, Some(Utc::now()))?;

        debug!(
            total_time_waiting = ?time_waiting,
            "finished fetching package index from thunderstore"
        );

        Ok(())
    }

    /// Returns version information for a package, if found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_core::PackageId;
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// let versions = index.version_info(&PackageId::new("Author-Pkg")).unwrap();
    /// ```
    pub fn version_info(
        &self,
        id: &PackageId,
    ) -> Result<Option<Vec<loadsmith_registry::VersionInfo>>> {
        let db = self.db.lock();

        let versions = db
            .prepare(include_str!("queries/select_version_info.sql"))?
            .query_map(rusqlite::params![id.as_str()], |row| {
                let version: Version = row.get::<_, String>(0)?.parse().unwrap();

                Ok(loadsmith_registry::VersionInfo { version })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if versions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(versions))
        }
    }

    /// Resolves a package reference to download URL, file size, and
    /// dependencies.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_core::{PackageId, PackageRef, Version};
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// let resolved = index
    ///     .resolve(&PackageRef::new(
    ///         PackageId::new("Author-Pkg"),
    ///         Version::new(1, 0, 0),
    ///     ))
    ///     .unwrap();
    /// ```
    pub fn resolve(
        &self,
        ref_: &PackageRef,
    ) -> Result<Option<loadsmith_registry::ResolvedVersion>> {
        let db = self.db.lock();

        let resolved = db
            .prepare(include_str!("queries/select_resolved.sql"))?
            .query_one(
                rusqlite::params![ref_.id().as_str(), ref_.version().to_string()],
                |row| {
                    let url = row.get::<_, String>(0)?;
                    let size = row.get::<_, i64>(1)?;

                    let deps_json = row.get::<_, String>(2)?;
                    let deps: Vec<VersionIdent> = serde_json::from_str(&deps_json).unwrap();

                    let categories_json = row.get::<_, String>(3)?;
                    let categories: Vec<String> = serde_json::from_str(&categories_json).unwrap();
                    let is_modpack = categories.contains(&super::MODPACK_CATEGORY.to_string());
                    let deps = super::dependencies_from_idents(&deps, is_modpack);

                    let url = loadsmith_core::FileUrl::try_from_url(&url).unwrap();

                    Ok(loadsmith_registry::ResolvedVersion {
                        url,
                        deps,
                        size: Some(size as u64),
                        checksum: None,
                    })
                },
            )?;

        Ok(Some(resolved))
    }

    /// Searches for packages by name, optionally scoped to a community.
    ///
    /// Results are ordered by pinned status, rating score, and total
    /// downloads.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loadsmith_thunderstore::sqlite::SqliteIndex;
    /// use thunderstore::Client;
    ///
    /// let db = rusqlite::Connection::open_in_memory().unwrap();
    /// let index = SqliteIndex::connect(Client::new(), db).unwrap();
    /// let results = index.search_packages("BepInEx", Some("rounds")).unwrap();
    /// ```
    pub fn search_packages(&self, query: &str, community: Option<&str>) -> Result<Vec<PackageId>> {
        let db = self.db.lock();

        let packages = db
            .prepare(include_str!("queries/search_packages.sql"))?
            .query_map(
                rusqlite::params![format!("%{}%", query), community],
                |row| {
                    let id: String = row.get(0)?;
                    Ok(PackageId::new(&id))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn fetch_and_resolve_version() {
        const COMMUNITY: &str = "rounds";

        let client = thunderstore::Client::new();
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let index = SqliteIndex::connect(client, db).unwrap();

        assert!(index.community_metadata(COMMUNITY).unwrap().is_none());

        index.update("rounds").await.unwrap();

        assert!(
            index
                .community_metadata(COMMUNITY)
                .unwrap()
                .and_then(|c| c.last_updated)
                .is_some()
        );

        let rounds_with_friends = PackageId::new("olavim-RoundsWithFriends");

        let versions = index.version_info(&rounds_with_friends).unwrap();
        assert!(versions.is_some());
        println!("{versions:#?}");

        let resolved = index
            .resolve(&PackageRef::new(
                rounds_with_friends,
                versions.unwrap().pop().unwrap().version,
            ))
            .unwrap();
        assert!(resolved.is_some());
        println!("{resolved:#?}");

        let versions = index.version_info(&PackageId::new("fake_package")).unwrap();

        assert!(versions.is_none());
    }
}
