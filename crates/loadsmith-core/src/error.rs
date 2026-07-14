use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid version part: {0}")]
    InvalidVersionPart(ParseIntError),

    #[error("invalid version format")]
    InvalidVersionFormat,

    #[error("invalid package reference format")]
    InvalidPackageRefFormat,

    #[error("unknown checksum algorithm: {0}")]
    UnknownAlgorithm(String),

    #[error("invalid checksum format")]
    InvalidChecksumFormat,

    #[error("invalid blake3 hex value")]
    InvalidBlake3Hex(#[source] blake3::HexError),
}

pub type Result<T> = std::result::Result<T, Error>;
