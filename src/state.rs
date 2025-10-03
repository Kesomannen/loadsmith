use std::{collections::HashMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, IoResultExt, Result};

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
            .wrap_err(&path, "reading profile state file")
            .and_then(|str| serde_json::from_str(&str).map_err(Error::Json))
            .unwrap_or_default();

        Self { path, state }
    }

    pub fn commit(&self) -> Result<()> {
        let parent = self
            .path
            .parent()
            .expect("state file must have parent directory");

        fs::create_dir_all(&parent).wrap_err(parent, "creating parent directory")?;
        let str = serde_json::to_string_pretty(&self.state)?;
        fs::write(&self.path, str).wrap_err(&self.path, "writing state file")?;

        Ok(())
    }
}
