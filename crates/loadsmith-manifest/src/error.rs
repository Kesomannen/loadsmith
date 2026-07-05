use loadsmith_core::{PackageId, PackageRef, VersionRange};

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

    #[error("package is not installed")]
    PackageNotInstalled,

    #[error("package already installed")]
    PackageAlreadyInstalled,
}

pub type Result<T> = std::result::Result<T, Error>;
