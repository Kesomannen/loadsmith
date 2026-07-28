use std::{fmt::Display, io::Read, str::FromStr};

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Checksum {
    Blake3(blake3::Hash),
    Sha256([u8; 32]),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Blake3,
    Sha256,
}

impl Checksum {
    pub fn blake3(hash: blake3::Hash) -> Self {
        Self::from(hash)
    }

    pub fn sha256(hash: [u8; 32]) -> Self {
        Self::Sha256(hash)
    }

    pub fn compute<R>(mut reader: R, algorithm: ChecksumAlgorithm) -> Result<Self>
    where
        R: Read,
    {
        match algorithm {
            ChecksumAlgorithm::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                std::io::copy(&mut reader, &mut hasher)?;
                Ok(Checksum::Blake3(hasher.finalize()))
            }
            ChecksumAlgorithm::Sha256 => {
                let mut hasher = digest_io::IoWrapper(sha2::Sha256::new());
                std::io::copy(&mut reader, &mut hasher)?;

                let array = hasher.0.finalize().into();

                Ok(Checksum::Sha256(array))
            }
        }
    }

    pub fn algorithm(&self) -> ChecksumAlgorithm {
        match self {
            Checksum::Blake3(_) => ChecksumAlgorithm::Blake3,
            Checksum::Sha256(_) => ChecksumAlgorithm::Sha256,
        }
    }

    pub fn without_algorithm(&self) -> WithoutAlgorithm<'_> {
        WithoutAlgorithm(self)
    }

    pub fn from_value_str(value: &str, algorithm: ChecksumAlgorithm) -> Result<Self> {
        match algorithm {
            ChecksumAlgorithm::Blake3 => {
                let hash = blake3::Hash::from_hex(value).map_err(Error::InvalidBlake3Hex)?;
                Ok(Checksum::Blake3(hash))
            }
            ChecksumAlgorithm::Sha256 => {
                let mut hash = [0u8; 32];
                hex::decode_to_slice(value, &mut hash).map_err(Error::InvalidSha256Hex)?;
                Ok(Checksum::Sha256(hash))
            }
        }
    }
}

impl From<blake3::Hash> for Checksum {
    fn from(hash: blake3::Hash) -> Self {
        Checksum::Blake3(hash)
    }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm(), self.without_algorithm())
    }
}

impl FromStr for Checksum {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (algorithm, value) = s.split_once(':').ok_or(Error::InvalidChecksumFormat)?;
        Checksum::from_value_str(value, algorithm.parse()?)
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

impl Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let algorithm = match self {
            ChecksumAlgorithm::Blake3 => "blake3",
            ChecksumAlgorithm::Sha256 => "sha256",
        };

        write!(f, "{algorithm}")
    }
}

impl FromStr for ChecksumAlgorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "blake3" => Ok(ChecksumAlgorithm::Blake3),
            "sha256" => Ok(ChecksumAlgorithm::Sha256),
            algo => Err(Error::UnknownAlgorithm(algo.to_string())),
        }
    }
}

pub struct WithoutAlgorithm<'a>(&'a Checksum);

impl Display for WithoutAlgorithm<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Checksum::Blake3(hash) => write!(f, "{}", hash.to_hex()),
            Checksum::Sha256(hash) => write!(f, "{}", hex::encode(hash)),
        }
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
