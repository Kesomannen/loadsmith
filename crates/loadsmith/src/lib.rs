//! A modular, interface-agnostic Rust library for mod manager implementations.
//!
//! The `loadsmith` facade crate re-exports the most important types from each
//! sub-crate so that consumers can depend on a single crate instead of many.
//!
//! # Organisation
//!
//! | Module | Source crate | Purpose |
//! |--------|-------------|---------|
//! | `core` | `loadsmith-core` | Core types (`PackageId`, `Version`, etc.) |
//! | `install` | `loadsmith-install` | Package installation rules |
//! | `loader` | `loadsmith-loader` | Mod loader definitions |
//! | `manifest` | `loadsmith-manifest` | Profile lockfile management |
//! | `platform` | `loadsmith-platform` | Game/platform detection |
//! | `registry` | `loadsmith-registry` | Registry trait and types |
//! | `thunderstore` | `loadsmith-thunderstore` | Thunderstore registry |
//! | `github` | `loadsmith-github` | GitHub Releases registry |
//! | `util` | `loadsmith-util` | Hashing and file utilities |

pub use loadsmith_core::{
    Checksum, ChecksumAlgorithm, Dependency, InstalledFile, InstalledPackage, PackageId,
    PackageRef, Version, VersionReq,
};

/// Core types such as [`PackageId`], [`Version`], and [`Checksum`].
///
/// Re-exported from `loadsmith-core`.
pub mod core {
    pub use loadsmith_core::{Error, Result};
}

pub use loadsmith_install::{
    InstallRule, InstallRuleset, OwnedInstallRuleset, extract, install, uninstall,
};

/// Package installation and extraction logic.
///
/// Re-exported from `loadsmith-install`.
pub mod install {
    pub use loadsmith_install::{ConflictStrategy, Error, GlobRule, Result, RouteRule};
}

pub use loadsmith_loader::{LaunchArgs, LaunchContext, Loader};

/// Mod loader definitions (BepInEx, MelonLoader, GDWeave, etc.).
///
/// Re-exported from `loadsmith-loader`.
pub mod loader {
    pub use loadsmith_loader::{
        BepInEx, BepisLoader, Error, GDWeave, Lovely, MelonLoader, Northstar, Result,
        ReturnOfModding, Rivet, Shimloader,
    };
}

pub use loadsmith_manifest::{
    LockedPackage, Lockfile, PackageStore, PackageStoreEntry, ProfileState, ProfileStateData,
    download_and_extract, resolve,
};

/// Profile lockfile management and dependency resolution.
///
/// Re-exported from `loadsmith-manifest`.
pub mod manifest {
    pub use loadsmith_manifest::{Diff, Diffable, Error, Result};
}

pub use loadsmith_platform::Platform;

/// Game-platform detection and launch-command generation.
///
/// Re-exported from `loadsmith-platform`.
pub mod platform {
    pub use loadsmith_platform::{Error, Result, find_executables, guess_proton, try_guess_proton};
}

pub use loadsmith_registry::{LocalRegistry, Registry, RegistrySet, offline::OfflineRegistry};

/// The [`Registry`] trait and related types for resolving package versions.
///
/// Re-exported from `loadsmith-registry`.
pub mod registry {
    pub use loadsmith_registry::{Error, ResolvedVersion, Result, VersionInfo, local, offline};
}

pub use loadsmith_thunderstore::{ThunderstoreRegistry, r2z};

/// Integration with the [Thunderstore](https://thunderstore.io/) mod registry.
///
/// Re-exported from `loadsmith-thunderstore`.
pub mod thunderstore {
    pub use loadsmith_thunderstore::{
        Error, PackageIdExt, PackageRefExt, Result, distribution_into_platform,
        in_memory::{self, InMemoryIndex},
        r2_config_to_loader,
        sqlite::{self, SqliteIndex},
    };
}

pub use loadsmith_github::GithubRegistry;

/// GitHub Releases registry for resolving mod packages.
///
/// Re-exported from `loadsmith-github`.
pub mod github {
    pub use loadsmith_github::{Error, Result};
}

/// Internal utility functions for hashing and file operations.
///
/// Re-exported from `loadsmith-util`.
pub mod util {
    pub use loadsmith_util::{
        copy_dir, create_parent_dirs, exact_version_eq, remove_empty_parents,
    };
}
