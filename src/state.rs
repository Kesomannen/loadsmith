use std::{collections::HashMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

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
    pub fn new(path: PathBuf) -> Self {
        let state = fs::read_to_string(&path)
            .map_err(|err| Error::wrap_io(err, &path))
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
