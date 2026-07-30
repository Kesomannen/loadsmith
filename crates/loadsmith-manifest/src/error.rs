use loadsmith_core::{PackageId, PackageRef, VersionReq};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("walkdir error")]
    Walkdir(#[from] walkdir::Error),

    #[error(transparent)]
    Install(#[from] loadsmith_install::Error),

    #[error(transparent)]
    Registry(#[from] loadsmith_registry::Error),

    #[error("unknown registry: {0}")]
    UnknownRegistry(String),

    #[error("no available version found for package {0} matching requirement {1}")]
    NoAvailableVersion(PackageId, VersionReq),

    #[error("failed to get versions for package {id} from source {source}")]
    VersionInfo {
        id: PackageId,
        source: String,
        #[source]
        err: loadsmith_registry::Error,
    },

    #[error("failed to resolve package {ref_} from source {source}")]
    Resolve {
        ref_: PackageRef,
        source: String,
        #[source]
        err: loadsmith_registry::Error,
    },

    #[error("failed to revalidate package {ref_} from source {source}")]
    Revalidate {
        ref_: PackageRef,
        source: String,
        #[source]
        err: loadsmith_registry::Error,
    },

    #[error("path contains non-UTF-8 characters")]
    NonUtf8Path,

    #[error("package is not installed")]
    PackageNotInstalled,

    #[error("package already installed")]
    PackageAlreadyInstalled,

    #[error("invalid package store entry path")]
    InvalidPackageStoreEntryPath,

    #[error("invalid package store entry version")]
    InvalidPackageStoreEntryVersion(#[source] semver::Error),

    #[error("invalid package store entry checksum")]
    InvalidPackageStoreEntryChecksum(#[source] loadsmith_core::Error),

    #[error("package store entry already exists")]
    PackageStoreEntryAlreadyExists,

    #[error("download task failed")]
    Download(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, Error>;
