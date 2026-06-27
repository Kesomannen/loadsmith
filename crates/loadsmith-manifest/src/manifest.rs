use std::collections::BTreeMap;

use loadsmith_core::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub profile: Profile,
    pub mods: Dependencies,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    #[serde(default, rename = "version")]
    pub version: Option<u32>,
    pub game: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependencies(BTreeMap<String, Dependency>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Dependency {
    Simple(Version),
    Detailed {
        version: Version,
        #[serde(default)]
        source: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_manifest() {
        let manifest_str = r#"
[profile]
version = 1
game = "valheim"

[mods]
author-name = "1.2.3"
"author/name-2" = { version = "4.5.6", source = "github" }
"#;

        let manifest: Manifest =
            toml::from_str(manifest_str).expect("failed to deserialize manifest");

        assert_eq!(manifest.profile.version, Some(1));
        assert_eq!(manifest.profile.game, "valheim");
        assert_eq!(
            manifest.mods.0.get("author-name"),
            Some(&Dependency::Simple(Version::new(1, 2, 3)))
        );
        assert_eq!(
            manifest.mods.0.get("author/name-2"),
            Some(&Dependency::Detailed {
                version: Version::new(4, 5, 6),
                source: Some("github".to_string()),
            })
        );
    }
}
