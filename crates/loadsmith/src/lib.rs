//! A modular, interface-agnostic Rust library for mod manager implementations,
//! primarily focused on the [Thunderstore](https://thunderstore.io) ecosystem.
//!
//! The `loadsmith` facade crate re-exports the most important types from each
//! sub-crate so that consumers can depend on a single crate instead of many.

pub use loadsmith_core::{
    Checksum, ChecksumAlgorithm, InstalledFile, InstalledPackage, PackageId, PackageRef, Version,
    VersionReq,
};

/// [`Error`](core::Error) and [`Result`](core::Result) types re-exported from `loadsmith-core`.
pub mod core {
    pub use loadsmith_core::{Error, Result};
}

pub use loadsmith_install::{extract, install, uninstall};

/// Lower level package installation and uninstallation APIs.
///
/// Re-exported from `loadsmith-install`.
pub mod install {
    pub use loadsmith_install::{ConflictStrategy, Error, Result, rule};
}

pub use loadsmith_loader::{LaunchArgs, LaunchContext, Loader};

/// Mod loader definitions ([BepInEx](loader::BepInEx), [MelonLoader](loader::MelonLoader), [GDWeave](loader::GDWeave), etc.).
///
/// Re-exported from `loadsmith-loader`. Also includes the [`Error`](loader::Error) and [`Result`](loader::Result) types from the same crate.
pub mod loader {
    pub use loadsmith_loader::{
        BepInEx, BepisLoader, Error, GDWeave, Lovely, MelonLoader, Northstar, Result,
        ReturnOfModding, Rivet, Shimloader,
    };
}

pub use loadsmith_manifest::{
    PackageStore, PackageStoreEntry, ProfileState, ProfileStateData, download_and_extract,
};

/// Profile lockfile, state management and dependency resolution.
///
/// Re-exported from `loadsmith-manifest`. Also includes the [`Error`](manifest::Error) and [`Result`](manifest::Result) types from the same crate.
pub mod manifest {
    pub use loadsmith_manifest::{Diff, Diffable, Error, LockedPackage, Lockfile, Result, resolve};
}

pub use loadsmith_platform::Platform;

/// Game and platform detection.
///
/// Re-exported from `loadsmith-platform`. Also includes the [`Error`](platform::Error) and [`Result`](platform::Result) types from the same crate.
pub mod platform {
    pub use loadsmith_platform::{Error, Result, find_executables, guess_proton, try_guess_proton};
}

pub use loadsmith_registry::{Dependency, Registry, RegistrySet};

/// Platform-independent registry implementations and registry-related types.
///
/// Re-exported from `loadsmith-registry`. Also includes the [`Error`](registry::Error) and [`Result`](registry::Result) types from the same crate.
pub mod registry {
    pub use loadsmith_registry::{
        Error, LocalRegistry, ResolvedVersion, Result, VersionInfo, local, offline,
        offline::OfflineRegistry,
    };
}

/// Integration with the [Thunderstore](https://thunderstore.io/) modding platform.
///
/// Contains the following:
/// - [`ThunderstoreRegistry`](thunderstore::ThunderstoreRegistry), a [`Registry`] implementation for resolving packages from a Thunderstore index.
/// - [`InMemoryIndex`](thunderstore::in_memory::InMemoryIndex) and [`SqliteIndex`](thunderstore::sqlite::SqliteIndex)
///   for caching and indexing Thunderstore packages, used by the registry.
/// - Methods and traits for converting Thunderstore metadata into `loadsmith` types.
/// - Tools to convert to and from Thunderstore's r2z profile format.
///
/// Re-exported from `loadsmith-thunderstore`. Also includes the [`Error`](thunderstore::Error) and [`Result`](thunderstore::Result) types from the same crate.
pub mod thunderstore {
    pub use loadsmith_thunderstore::{
        Error, PackageIdExt, PackageRefExt, Result, ThunderstoreRegistry,
        distribution_into_platform,
        in_memory::{self, InMemoryIndex},
        r2_config_to_loader, r2z,
        sqlite::{self, SqliteIndex},
    };
}

/// Integration with Github-hosted mods
///
/// Contains [`GithubRegistry`](github::GithubRegistry), a [`Registry`] implementation for resolving packages from Github Releases.
///
/// Re-exported from `loadsmith-github`. Also includes the [`Error`](github::Error) and [`Result`](github::Result) types from the same crate.
pub mod github {
    pub use loadsmith_github::{Error, GithubRegistry, Result};
}
