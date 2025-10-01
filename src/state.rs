use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{Error, Result, wrap_io_err};

pub struct ProfileStateHandle {
    pub path: PathBuf,
    pub state: ProfileState,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileState {
    pub file_map: HashMap<PathBuf, String>,
}

impl ProfileStateHandle {
    pub fn new(profile_root: &Path) -> Self {
        let mut path = profile_root.to_path_buf();

        path.push("_state");
        path.push("profile");
        path.set_extension("json");

        let state = fs::read_to_string(&path)
            .map_err(|err| wrap_io_err(err, &path))
            .and_then(|str| serde_json::from_str(&str).map_err(Error::Json))
            .unwrap_or_default();

        Self { path, state }
    }

    pub fn commit(&self) -> Result<()> {
        fs::create_dir_all(self.path.parent().unwrap())?;
        let str = serde_json::to_string(&self.state)?;
        fs::write(&self.path, str)?;

        Ok(())
    }
}
