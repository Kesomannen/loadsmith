use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Checksum {
    Blake3(blake3::Hash),
}

impl Checksum {
    pub fn blake3(hash: blake3::Hash) -> Self {
        Self::from(hash)
    }
}

impl From<blake3::Hash> for Checksum {
    fn from(hash: blake3::Hash) -> Self {
        Checksum::Blake3(hash)
    }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Checksum::Blake3(hash) => write!(f, "blake3:{}", hash.to_hex()),
        }
    }
}

impl FromStr for Checksum {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (algorithm, value) = s.split_once(':').ok_or(Error::InvalidVersionFormat)?;

        match algorithm {
            "blake3" => {
                let hash = blake3::Hash::from_hex(value).map_err(Error::InvalidBlake3Hex)?;
                Ok(Checksum::Blake3(hash))
            }
            algo => Err(Error::UnknownAlgorithm(algo.to_string())),
        }
    }
}

impl From<Checksum> for String {
    fn from(checksum: Checksum) -> Self {
        checksum.to_string()
    }
}

impl TryFrom<String> for Checksum {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // blake3 hash of "Hello, world!"
    const BLAKE3_HELLO_WORLD: &str =
        "ede5c0b10f2ec4979c69b52f61e42ff5b413519ce09be0f14d098dcfe5f6f98d";

    #[test]
    fn checksum_display() {
        let hash = blake3::hash(b"Hello, world!");
        let checksum = Checksum::Blake3(hash);
        assert_eq!(checksum.to_string(), format!("blake3:{BLAKE3_HELLO_WORLD}"));
    }

    #[test]
    fn checksum_from_str() {
        let checksum_str = format!("blake3:{BLAKE3_HELLO_WORLD}");
        let checksum = Checksum::from_str(&checksum_str).unwrap();

        let hash = blake3::hash(b"Hello, world!");
        assert_eq!(checksum, Checksum::Blake3(hash));
    }

    #[test]
    fn checksum_from_str_invalid_algorithm() {
        let checksum_str = "unknown:abcdef";
        let result = Checksum::from_str(&checksum_str);
        assert!(result.is_err());
    }

    #[test]
    fn checksum_from_str_invalid_format() {
        let checksum_str = "blake3abcdef";
        let result = Checksum::from_str(&checksum_str);
        assert!(result.is_err());
    }

    #[test]
    fn checksum_serde_json() {
        let hash = blake3::hash(b"Hello, world!");
        let checksum = Checksum::Blake3(hash);

        let json = serde_json::to_string(&checksum).unwrap();
        assert_eq!(json, format!("\"blake3:{BLAKE3_HELLO_WORLD}\""));

        let deserialized: Checksum = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, checksum);
    }
}
