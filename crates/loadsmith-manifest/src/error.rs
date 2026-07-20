use loadsmith_core::{PackageId, PackageRef, VersionRange};

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

    #[error("no available version found for package {0} matching range {1}")]
    NoAvailableVersion(PackageId, VersionRange),

    #[error("failed to get versions for package {id}")]
    VersionInfo {
        id: PackageId,
        #[source]
        err: loadsmith_registry::Error,
    },

    #[error("failed to resolve package {ref_}")]
    Resolve {
        ref_: PackageRef,
        #[source]
        err: loadsmith_registry::Error,
    },

    #[error("failed to revalidate package {ref_}")]
    Revalidate {
        ref_: PackageRef,
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
    InvalidPackageStoreEntryVersion(#[source] loadsmith_core::Error),

    #[error("invalid package store entry checksum")]
    InvalidPackageStoreEntryChecksum(#[source] loadsmith_core::Error),

    #[error("package store entry already exists")]
    PackageStoreEntryAlreadyExists,
}

pub type Result<T> = std::result::Result<T, Error>;
