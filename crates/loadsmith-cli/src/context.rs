use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

use crate::{Result, lockfile::Lockfile, manifest::Manifest};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Context {
    pub working_dir: PathBuf,
}

impl Context {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    pub(crate) fn path(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.working_dir.join(relative_path)
    }

    pub(crate) fn read_toml<T: DeserializeOwned>(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<T> {
        let content = fs::read_to_string(self.path(relative_path))?;
        let value = toml::from_str(&content)?;
        Ok(value)
    }

    pub(crate) fn write_toml<T: serde::Serialize>(
        &self,
        relative_path: impl AsRef<Path>,
        value: &T,
    ) -> Result {
        let content = toml::to_string(value)?;
        fs::write(self.path(relative_path), content)?;
        Ok(())
    }

    pub(crate) fn read_manifest(&self) -> Result<Manifest> {
        self.read_toml(Manifest::FILE_NAME)
    }

    pub(crate) fn read_lockfile_option(&self) -> Result<Option<Lockfile>> {
        let path = self.path(Lockfile::FILE_NAME);
        if path.exists() {
            self.read_lockfile().map(Some)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn read_lockfile(&self) -> Result<Lockfile> {
        self.read_toml(Lockfile::FILE_NAME)
    }
}
