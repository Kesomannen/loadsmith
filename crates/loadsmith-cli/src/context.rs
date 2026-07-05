use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use loadsmith::{
    manifest::{Lockfile, ProfileState, ProfileStateData},
    registry::RegistrySet,
    thunderstore::sqlite::SqliteIndex,
};
use serde::de::DeserializeOwned;

use crate::{Result, manifest::Manifest, profile::Profile};

#[derive(Debug)]
pub struct Context {
    pub(crate) http: reqwest::Client,
    pub(crate) thunderstore: thunderstore::Client,
    pub(crate) registry_set: RegistrySet,
    pub(crate) index: SqliteIndex,
    pub(crate) working_dir: PathBuf,
}

impl Context {
    const MANIFEST_FILE_NAME: &str = "loadsmith.toml";
    const LOCKFILE_FILE_NAME: &str = "loadsmith.lock";
    const PROFILE_STATE_FILE_NAME: &str = "_state/profile.json";

    pub fn new(
        http: reqwest::Client,
        thunderstore: thunderstore::Client,
        registry_set: RegistrySet,
        index: SqliteIndex,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            http,
            thunderstore,
            registry_set,
            index,
            working_dir,
        }
    }

    fn path(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.working_dir.join(relative_path)
    }

    fn read<T: DeserializeOwned, F>(&self, relative_path: impl AsRef<Path>, parse: F) -> Result<T>
    where
        F: FnOnce(&str) -> Result<T>,
    {
        let content = fs::read_to_string(self.path(relative_path))?;
        let value = parse(&content)?;
        Ok(value)
    }

    fn write(&self, relative_path: impl AsRef<Path>, content: &str) -> Result {
        let path = relative_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(self.path(relative_path), content)?;
        Ok(())
    }

    fn read_toml<T: DeserializeOwned>(&self, relative_path: impl AsRef<Path>) -> Result<T> {
        self.read(relative_path, |content| {
            toml::from_str(content).context("failed to parse TOML")
        })
    }

    fn write_toml<T: serde::Serialize>(
        &self,
        relative_path: impl AsRef<Path>,
        value: &T,
    ) -> Result {
        self.write(relative_path, &toml::to_string_pretty(value)?)
    }

    fn read_json<T: DeserializeOwned>(&self, relative_path: impl AsRef<Path>) -> Result<T> {
        self.read(relative_path, |content| {
            serde_json::from_str(content).context("failed to parse JSON")
        })
    }

    fn write_json<T: serde::Serialize>(
        &self,
        relative_path: impl AsRef<Path>,
        value: &T,
    ) -> Result {
        self.write(relative_path, &serde_json::to_string_pretty(value)?)
    }

    fn read_manifest(&self) -> Result<Manifest> {
        self.read_toml(Self::MANIFEST_FILE_NAME)
            .context("failed to read manifest")
    }

    fn read_lockfile_option(&self) -> Result<Option<Lockfile>> {
        let path = self.path(Self::LOCKFILE_FILE_NAME);
        if path.exists() {
            self.read_lockfile()
                .context("failed to read lockfile")
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_lockfile(&self) -> Result<Lockfile> {
        self.read_json(Self::LOCKFILE_FILE_NAME)
    }

    fn read_profile_state(&self) -> Result<ProfileState> {
        let path = self.path(Self::PROFILE_STATE_FILE_NAME);
        let data = if path.exists() {
            self.read_json(Self::PROFILE_STATE_FILE_NAME)?
        } else {
            ProfileStateData::default()
        };

        Ok(ProfileState::new(self.working_dir.clone(), data))
    }

    pub(crate) fn read_profile(&self) -> Result<Profile> {
        let manifest = self.read_manifest()?;
        let lockfile = self.read_lockfile_option()?.unwrap_or_default();
        let state = self.read_profile_state()?;

        Ok(Profile {
            manifest,
            lockfile,
            state,
        })
    }

    fn write_manifest(&self, manifest: &Manifest) -> Result {
        self.write_toml(Self::MANIFEST_FILE_NAME, manifest)
            .context("failed to write manifest")
    }

    fn write_lockfile(&self, lockfile: &Lockfile) -> Result {
        self.write_json(Self::LOCKFILE_FILE_NAME, lockfile)
            .context("failed to write lockfile")
    }

    fn write_profile_state(&self, state: &ProfileState) -> Result {
        self.write_json(Self::PROFILE_STATE_FILE_NAME, state.data())
            .context("failed to write profile state")
    }

    pub(crate) fn write_profile(&self, profile: &Profile) -> Result {
        self.write_manifest(&profile.manifest)?;
        self.write_lockfile(&profile.lockfile)?;
        self.write_profile_state(&profile.state)?;
        Ok(())
    }
}
