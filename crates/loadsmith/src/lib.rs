pub use loadsmith_core::{
    Dependency, InstalledFile, InstalledPackage, PackageId, PackageRef, Version, VersionRange,
};
pub mod core {
    pub use loadsmith_core::{Error, Result};
}
pub use loadsmith_install::{
    InstallRule, InstallRuleset, OwnedInstallRuleset, extract, extract_zip, install, uninstall,
};
pub mod install {
    pub use loadsmith_install::{ConflictStrategy, Error, GlobRule, Result, RouteRule};
}
pub use loadsmith_loader::{LaunchArgs, LaunchContext, Loader};
pub mod loader {
    pub use loadsmith_loader::{
        BepInEx, BepisLoader, Error, MelonLoader, Result, ReturnOfModding, Shimloader,
    };
}
pub use loadsmith_manifest::{LockedPackage, Lockfile, ProfileState, ProfileStateData, resolve};
pub mod manifest {
    pub use loadsmith_manifest::{Diff, Diffable, Error, Result};
}
pub use loadsmith_platform::Platform;
pub mod platform {
    pub use loadsmith_platform::{Error, Result, find_executables, guess_proton, try_guess_proton};
}
pub use loadsmith_registry::{
    Registry, RegistrySet, local::LocalRegistry, offline::OfflineRegistry,
};
pub mod registry {
    pub use loadsmith_registry::{Error, ResolvedVersion, Result, VersionInfo, offline};
}
pub use loadsmith_thunderstore::{ThunderstoreRegistry, r2z};
pub mod thunderstore {
    pub use loadsmith_thunderstore::{
        Error, PackageIdExt, PackageRefExt, Result, distribution_into_platform,
        in_memory::{self, InMemoryIndex},
        r2_config_to_loader,
        sqlite::{self, SqliteIndex},
    };
}
