use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub profile: Profile,
    pub mods: Mods,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub game: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mods(pub HashMap<String, Mod>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mod {
    Simple(loadsmith::core::Version),
}

impl Manifest {
    pub const FILE_NAME: &str = "loadsmith.toml";

    pub fn new(profile: Profile, mods: Mods) -> Self {
        Self { profile, mods }
    }
}

impl Profile {
    pub fn new(game: impl Into<String>) -> Self {
        Self { game: game.into() }
    }
}

impl Mods {
    pub fn new(mods: HashMap<String, Mod>) -> Self {
        Self(mods)
    }
}

impl From<Mods> for loadsmith::manifest::Dependencies {
    fn from(value: Mods) -> Self {
        loadsmith::manifest::Dependencies::from(
            value
                .0
                .into_iter()
                .map(|(name, mod_)| {
                    let package_id = loadsmith::core::PackageId::new(name);
                    let dependency = mod_.into();
                    (package_id, dependency)
                })
                .collect::<HashMap<_, _>>(),
        )
    }
}

impl From<Mod> for loadsmith::manifest::Dependency {
    fn from(value: Mod) -> Self {
        match value {
            Mod::Simple(version) => loadsmith::manifest::Dependency::new(version),
        }
    }
}
