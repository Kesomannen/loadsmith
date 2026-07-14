#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Github API error")]
    Github(#[from] octocrab::Error),

    #[error("Glob error")]
    Glob(#[from] globset::Error),

    #[error("invalid package ID format")]
    InvalidPackageIdFormat,

    #[error("invalid release tag format")]
    InvalidReleaseTagFormat { tag: String },

    #[error("invalid release tag version")]
    InvalidReleaseTagVersion {
        tag: String,
        #[source]
        err: loadsmith_core::Error,
    },

    #[error("package not found")]
    PackageNotFound,

    #[error("package version not found")]
    VersionNotFound,

    #[error("no matching assets found in release: {assets:?}")]
    NoMatchingAssets { assets: Vec<String> },

    #[error("multiple matching assets found in release: {matching_assets:?}")]
    MultipleMatchingAssets { matching_assets: Vec<String> },

    #[error("invalid release asset digest: {checksum}")]
    InvalidAssetDigest {
        checksum: String,
        #[source]
        err: loadsmith_core::Error,
    },
}

impl Error {
    pub(crate) fn map_github_404(err: octocrab::Error, f: impl FnOnce() -> Error) -> Self {
        match err {
            octocrab::Error::GitHub { source, .. }
                if source.status_code == http::StatusCode::NOT_FOUND =>
            {
                f()
            }
            err => Error::Github(err),
        }
    }
}

impl From<Error> for loadsmith_registry::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::PackageNotFound => loadsmith_registry::Error::PackageNotFound,
            Error::VersionNotFound => loadsmith_registry::Error::VersionNotFound,
            err => loadsmith_registry::Error::other(err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
