use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use sha2::Digest;
use walkdir::WalkDir;

use crate::{Error, Result};

/// A checksum value computed with a recognised algorithm (BLAKE3 or SHA-256).
///
/// ```rust
/// # use loadsmith_core::{Checksum, ChecksumAlgorithm};
/// # use std::io::Cursor;
/// let data = Cursor::new(b"BepInExPack_Valheim-5.4.2202.zip contents");
/// let ck = Checksum::compute(data, ChecksumAlgorithm::Blake3).unwrap();
/// assert_eq!(ck.algorithm().to_string(), "blake3");
///
/// let as_str = ck.to_string();
/// let parsed: Checksum = as_str.parse().unwrap();
/// assert_eq!(ck, parsed);
///
/// let ck = Checksum::from_value_str(
///     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
///     ChecksumAlgorithm::Sha256,
/// ).unwrap();
/// assert_eq!(ck.algorithm().to_string(), "sha256");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Checksum {
    Blake3(blake3::Hash),
    Sha256([u8; 32]),
}

/// The set of checksum algorithms this crate can handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Blake3,
    Sha256,
}

impl Checksum {
    /// Create a `Checksum::Blake3` from an already-computed `blake3::Hash`.
    pub fn blake3(hash: blake3::Hash) -> Self {
        Self::from(hash)
    }

    /// Create a `Checksum::Sha256` from a raw 32-byte array.
    pub fn sha256(hash: [u8; 32]) -> Self {
        Self::Sha256(hash)
    }

    /// Compute a checksum by reading a byte stream with the chosen algorithm.
    ///
    /// ```rust
    /// # use loadsmith_core::{Checksum, ChecksumAlgorithm};
    /// # use std::io::Cursor;
    /// let data = Cursor::new(b"some mod archive data");
    /// let ck = Checksum::compute(data, ChecksumAlgorithm::Sha256).unwrap();
    /// assert_eq!(ck.algorithm().to_string(), "sha256");
    /// ```
    pub fn compute<R>(mut reader: R, algorithm: ChecksumAlgorithm) -> std::io::Result<Self>
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

    pub fn compute_from_path(
        path: impl AsRef<Path>,
        algorithm: ChecksumAlgorithm,
    ) -> std::io::Result<Self> {
        fn hash_dir<T: Write>(mut hasher: T, path: &Path) -> std::io::Result<T> {
            WalkDir::new(path)
                .sort_by_file_name()
                .into_iter()
                .filter_map(|entry| entry.ok())
                .try_for_each(|entry| {
                    let file_name = entry.file_name().as_encoded_bytes();
                    hasher.write_all(file_name)?;

                    if entry.file_type().is_file() {
                        let mut file = File::open(entry.path()).map(BufReader::new)?;
                        std::io::copy(&mut file, &mut hasher)?;
                    }

                    Ok::<(), std::io::Error>(())
                })?;

            Ok(hasher)
        }

        let path = path.as_ref();
        if path.is_dir() {
            match algorithm {
                ChecksumAlgorithm::Blake3 => {
                    let hasher = blake3::Hasher::new();

                    let hasher = hash_dir(hasher, path.as_ref())?;

                    Ok(Checksum::Blake3(hasher.finalize()))
                }
                ChecksumAlgorithm::Sha256 => {
                    let hasher = digest_io::IoWrapper(sha2::Sha256::new());

                    let hasher = hash_dir(hasher, path.as_ref())?;

                    let array = hasher.0.finalize().into();
                    Ok(Checksum::Sha256(array))
                }
            }
        } else {
            let file = File::open(path).map(BufReader::new)?;
            Self::compute(file, algorithm)
        }
    }

    /// Return which algorithm this checksum was produced with.
    pub fn algorithm(&self) -> ChecksumAlgorithm {
        match self {
            Checksum::Blake3(_) => ChecksumAlgorithm::Blake3,
            Checksum::Sha256(_) => ChecksumAlgorithm::Sha256,
        }
    }

    /// Return a wrapper that displays only the hex portion (no algorithm prefix).
    pub fn without_algorithm(&self) -> WithoutAlgorithm<'_> {
        WithoutAlgorithm(self)
    }

    /// Parse a checksum from a raw hex string with an explicit algorithm.
    ///
    /// Useful when the algorithm and value are stored separately, or when
    /// you have already split `"<algo>:<hex>"` yourself.
    ///
    /// ```rust
    /// # use loadsmith_core::{Checksum, ChecksumAlgorithm};
    /// let ck = Checksum::from_value_str(
    ///     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ///     ChecksumAlgorithm::Sha256,
    /// ).unwrap();
    /// assert_eq!(ck.algorithm().to_string(), "sha256");
    /// ```
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

/// The hex-only portion of a [`Checksum`] (no algorithm prefix).
///
/// Created via [`Checksum::without_algorithm`].
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
