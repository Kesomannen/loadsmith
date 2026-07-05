use std::path::PathBuf;

use loadsmith::{registry::RegistrySet, thunderstore::sqlite::SqliteIndex};

#[derive(Debug)]
pub struct Context {
    pub(crate) http: reqwest::Client,
    pub(crate) thunderstore: thunderstore::Client,
    pub(crate) registry_set: RegistrySet,
    pub(crate) index: SqliteIndex,
    pub(crate) working_dir: PathBuf,
    pub(crate) home_dir: PathBuf,
}

impl Context {
    pub fn new(
        http: reqwest::Client,
        thunderstore: thunderstore::Client,
        registry_set: RegistrySet,
        index: SqliteIndex,
        working_dir: PathBuf,
        home_dir: PathBuf,
    ) -> Self {
        Self {
            http,
            thunderstore,
            registry_set,
            index,
            working_dir,
            home_dir,
        }
    }
}
