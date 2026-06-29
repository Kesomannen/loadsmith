use loadsmith_core::PackageId;
use loadsmith_registry::RegistryId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("registry error: {0}")]
    Registry(#[from] loadsmith_registry::Error),

    #[error("unknown registry: {0}")]
    UnknownRegistry(String),

    #[error("no default registry was set but a package was requested without a source")]
    NoDefaultRegistry,

    #[error("package not found: {package} in {registry}")]
    UnknownPackage {
        package: PackageId,
        registry: RegistryId,
    },

    #[error("no available version found for package: {id}")]
    NoAvailableVersion { id: PackageId },
}

pub type Result<T> = std::result::Result<T, Error>;
