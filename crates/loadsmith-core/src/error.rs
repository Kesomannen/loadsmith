use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid version part: {0}")]
    InvalidVersionPart(ParseIntError),

    #[error("invalid version format")]
    InvalidVersionFormat,

    #[error("invalid package reference format")]
    InvalidPackageRefFormat,
}

pub type Result<T> = std::result::Result<T, Error>;
