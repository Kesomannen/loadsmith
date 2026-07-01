use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(flatten)]
    pub inner: loadsmith::manifest::Lockfile,
}

impl Lockfile {
    pub const FILE_NAME: &str = "loadsmith.lock";

    pub fn new(inner: loadsmith::manifest::Lockfile) -> Self {
        Self { inner }
    }
}
