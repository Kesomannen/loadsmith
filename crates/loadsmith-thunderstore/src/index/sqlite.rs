use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use futures::{TryStreamExt, pin_mut};
use loadsmith_core::{PackageId, Version};
use parking_lot::Mutex;
use rusqlite::OptionalExtension;
use thunderstore::VersionIdent;
use tracing::{debug, trace};

use crate::Result;

#[derive(Debug, Clone)]
pub struct SqliteIndex {
    client: thunderstore::Client,
    db: Arc<Mutex<rusqlite::Connection>>,
}

#[derive(Debug, Clone)]
pub struct CommunityMetadata {
    pub community: String,
    pub last_updated: Option<DateTime<Utc>>,
}

impl SqliteIndex {
    pub fn connect(client: thunderstore::Client, db: rusqlite::Connection) -> Result<Self> {
        db.execute_batch(include_str!("queries/create_schema.sql"))?;
        Ok(Self {
            client,
            db: Arc::new(Mutex::new(db)),
        })
    }

    pub fn open(client: thunderstore::Client, db_path: impl AsRef<Path>) -> Result<Self> {
        let db = rusqlite::Connection::open(db_path)?;
        db.pragma_update(None, "journal_mode", "WAL")?;
        Self::connect(client, db)
    }

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

    pub fn resolve(
        &self,
        id: &PackageId,
        version: &Version,
    ) -> Result<Option<loadsmith_registry::ResolvedVersion>> {
        let db = self.db.lock();

        let resolved = db
            .prepare(include_str!("queries/select_resolved.sql"))?
            .query_one(rusqlite::params![id.as_str(), version.to_string()], |row| {
                let url = row.get::<_, String>(0)?;
                let size = row.get::<_, i64>(1)?;

                let deps_json = row.get::<_, String>(2)?;
                let deps: Vec<VersionIdent> = serde_json::from_str(&deps_json).unwrap();

                let categories_json = row.get::<_, String>(3)?;
                let categories: Vec<String> = serde_json::from_str(&categories_json).unwrap();
                let is_modpack = categories.contains(&super::MODPACK_CATEGORY.to_string());
                let deps = super::dependencies_from_idents(&deps, is_modpack);

                Ok(loadsmith_registry::ResolvedVersion {
                    url,
                    deps,
                    size: Some(size as u64),
                    checksum: None,
                })
            })?;

        Ok(Some(resolved))
    }

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
            .resolve(&rounds_with_friends, &versions.unwrap()[0].version)
            .unwrap();
        assert!(resolved.is_some());
        println!("{resolved:#?}");

        let versions = index.version_info(&PackageId::new("fake_package")).unwrap();

        assert!(versions.is_none());
    }
}
