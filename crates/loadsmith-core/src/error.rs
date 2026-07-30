#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("semver error")]
    Semver(#[from] semver::Error),

    #[error("URL parse error")]
    Url(#[from] url::ParseError),

    #[error("invalid package reference format")]
    InvalidPackageRefFormat,

    #[error("unknown checksum algorithm: {0}")]
    UnknownAlgorithm(String),

    #[error("invalid checksum format")]
    InvalidChecksumFormat,

    #[error("invalid blake3 hex value")]
    InvalidBlake3Hex(#[source] blake3::HexError),

    #[error("invalid sha256 hex value")]
    InvalidSha256Hex(#[source] hex::FromHexError),
}

pub type Result<T> = std::result::Result<T, Error>;
