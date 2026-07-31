/// Errors that can occur when interacting with the GitHub Releases registry.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error returned by the GitHub API client.
    #[error("Github API error")]
    Github(#[from] octocrab::Error),

    /// An error related to asset glob pattern parsing.
    #[error("Glob error")]
    Glob(#[from] globset::Error),

    /// The package ID does not contain a dash to split owner/repo.
    ///
    /// When no explicit `repo` metadata is provided, the package ID is
    /// expected to be in `owner-repo` format.
    #[error("invalid package ID format")]
    InvalidPackageIdFormat,

    /// The repository string in metadata is not in `owner/repo` format.
    #[error("invalid repository format: {0}")]
    InvalidRepositoryFormat(String),

    /// A release tag does not match the expected template.
    ///
    /// The tag must start with the template prefix and end with the template
    /// suffix, with the version string in between.
    #[error("invalid release tag format")]
    InvalidReleaseTagFormat { tag: String },

    /// The version portion of a release tag could not be parsed as a semver
    /// version.
    #[error("invalid release tag version")]
    InvalidReleaseTagVersion {
        tag: String,
        #[source]
        err: semver::Error,
    },

    /// The requested package was not found on GitHub.
    #[error("package not found")]
    PackageNotFound,

    /// The requested version of the package was not found.
    #[error("package version not found")]
    VersionNotFound,

    /// No release assets matched the configured glob pattern.
    #[error("no matching assets found in release: {assets:?}")]
    NoMatchingAssets { assets: Vec<String> },

    /// Multiple release assets matched the configured glob pattern; a single
    /// match is required.
    #[error("multiple matching assets found in release: {matching_assets:?}")]
    MultipleMatchingAssets { matching_assets: Vec<String> },

    /// A release asset's digest could not be parsed as a valid checksum.
    #[error("invalid release asset digest: {checksum}")]
    InvalidAssetDigest {
        checksum: String,
        #[source]
        err: loadsmith_core::Error,
    },
}

impl Error {
    /// Map a GitHub 404 error into a caller-specified error, passing through
    /// all other errors unchanged.
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

/// Alias for `Result<T, Error>` in the loadsmith-github crate.
pub type Result<T> = std::result::Result<T, Error>;
