# loadsmith — Project Design

> A modular, interface-agnostic Rust library for mod manager implementations.

---

## 1. Goals & Non-Goals

### Goals

- Correct, well-tested low-level install/remove/list logic for Thunderstore-style packages across all major mod loaders.
- Accurate launch argument generation for BepInEx, MelonLoader, UE4SS, GDWeave, and others on both Windows and Linux (including Proton).
- A Cargo-style manifest + lockfile system that can represent packages from multiple registries (Thunderstore, Curseforge, Nexus, GitHub Releases, local files).
- Thunderstore-specific features: API client, r2z portable profile format.
- A clean, documented public API that downstream crates (Tauri apps, Ratatui TUIs, CLI tools) can depend on without pulling in unneeded pieces.

### Non-Goals

- No UI of any kind.
- No opinion on how state is persisted beyond what the lockfile format requires.
- No bundling of frontend assets, IPC layers, or platform-specific packaging.

---

## 2. Workspace Structure

```
loadsmith/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── loadsmith/              # facade — re-exports the public API surface
│   ├── loadsmith-core/         # fundamental types, traits, errors
│   ├── loadsmith-install/      # install / remove / list file operations
│   ├── loadsmith-loader/       # loader definitions + launch arg generation
│   ├── loadsmith-manifest/     # manifest + lockfile + dependency resolver
│   ├── loadsmith-thunderstore/ # Thunderstore API client + r2z format
│   └── loadsmith-registry/     # registry abstraction + built-in sources
└── examples/
    ├── install-mod/
    └── thunderstore-fetch/
```

The facade crate `loadsmith` re-exports everything useful through a flat module hierarchy, so downstream crates can `use loadsmith::prelude::*` or reach into `loadsmith::thunderstore` as needed. Internal crates can be used directly for tighter dependency graphs.

---

## 3. Core Crate (`loadsmith-core`)

This crate defines the shared vocabulary. No I/O, no async. Everything else depends on it.

### 3.1 Package Identity

```rust
/// The stable identity of a package, registry-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageId {
    pub namespace: String,
    pub name: String,
}

impl fmt::Display for PackageId {
    // "Namespace-Name"
}

/// A reference to a specific version of a package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageRef {
    pub id: PackageId,
    pub version: Version,  // re-export of semver::Version
}
```

### 3.2 Platform & OS

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Steam,
    Epic,
    Xbox,
    Origin,
    Oculus,
    Direct,  // launched via absolute path, no launcher
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
}
```

### 3.3 Game

```rust
/// A game that loadsmith knows how to manage mods for.
/// Typically sourced from Thunderstore's game definitions or a local registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDef {
    pub id: String,              // e.g. "lethal-company"
    pub display_name: String,
    pub platform: Platform,
    pub steam_id: Option<u32>,
    pub loader: LoaderId,
    pub exe_name: String,        // e.g. "Lethal Company.exe"
}
```

### 3.4 Error Type

Each crate defines its own error enum with `thiserror`, and `loadsmith-core` provides a common `Error` for cross-crate wrapping. `anyhow` is kept out of the library — callers choose their own error strategy.

---

## 4. Install Crate (`loadsmith-install`)

This is the heart of the library. It answers the question: given a downloaded package archive and a profile directory, what files go where?

### 4.1 Profile

A profile is intentionally a thin type. It wraps a directory and a reference to the game definition; all operations are free functions, not methods. This keeps the type composable.

```rust
pub struct Profile {
    pub path: PathBuf,
    pub game: Arc<GameDef>,
}
```

### 4.2 Install Rules

Install rules are defined per-loader as a mapping from archive-relative path patterns to profile-relative destinations. They're data, not code, making them easy to test, serialise, and override.

```rust
pub struct InstallRule {
    /// Glob matched against archive entry paths after stripping the top-level folder.
    pub source: GlobPattern,
    /// Destination within the profile directory.
    pub dest: InstallTarget,
    /// Whether to preserve the matched sub-path or flatten into dest.
    pub flatten: bool,
}

pub enum InstallTarget {
    /// Profile root (e.g. for BepInEx itself, doorstop files).
    Root,
    /// Relative path inside the profile, e.g. "BepInEx/plugins/{pkg}".
    Relative(String),
}
```

The `{pkg}` token in `InstallTarget::Relative` is expanded to `Namespace-Name` at install time, giving each package its own subdirectory.

### 4.3 The Installer

```rust
/// Install a package from a zip archive into a profile.
/// Returns the list of files written to disk, relative to the profile root.
pub fn install(
    profile: &Profile,
    pkg: &PackageRef,
    archive: &[u8],
    rules: &[InstallRule],
    strategy: LinkStrategy,
) -> Result<Vec<PathBuf>, InstallError>

pub enum LinkStrategy {
    /// Hard-link files from a central cache (fast, low disk use — gale's current approach).
    HardLink { cache_dir: PathBuf },
    /// Full copy. Fallback for filesystems that don't support hard links.
    Copy,
}

/// Remove all files belonging to a package from a profile.
pub fn remove(
    profile: &Profile,
    pkg: &PackageRef,
    manifest: &InstalledManifest,
) -> Result<(), InstallError>

/// List all files belonging to a package, relative to the profile root.
pub fn list_files(
    profile: &Profile,
    pkg: &PackageRef,
    manifest: &InstalledManifest,
) -> Result<Vec<PathBuf>, InstallError>
```

### 4.4 Installed Manifest

To enable correct `remove` and `list_files`, loadsmith persists a per-package record of what was installed. This lives inside the profile directory.

```rust
/// Stored at `<profile>/.loadsmith/installed/<namespace>-<name>-<version>.json`
#[derive(Serialize, Deserialize)]
pub struct InstalledManifest {
    pub pkg: PackageRef,
    pub files: Vec<PathBuf>,   // profile-relative paths
    pub installed_at: DateTime<Utc>,
}
```

This replaces the need for scanning the profile directory to infer ownership — useful when two packages might share a directory but not files.

---

## 5. Loader Crate (`loadsmith-loader`)

Defines the `Loader` trait and concrete implementations for each supported loader.

### 5.1 Loader Trait

```rust
pub trait Loader: Send + Sync {
    fn id(&self) -> LoaderId;

    /// The install rules for a regular plugin package for this loader.
    fn plugin_rules(&self, pkg: &PackageRef) -> Vec<InstallRule>;

    /// Some loaders have special rules for their own core package
    /// (e.g. the BepInExPack goes to the profile root).
    fn core_package(&self) -> Option<PackageId>;
    fn core_rules(&self) -> Vec<InstallRule>;

    /// Arguments needed to launch the game with this loader active.
    fn launch_args(
        &self,
        profile: &Profile,
        os: Os,
        platform: Platform,
    ) -> LaunchArgs;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoaderId(pub String);  // "bepinex", "melonloader", "gdweave", ...
```

### 5.2 LaunchArgs

```rust
pub struct LaunchArgs {
    /// Environment variables to set before launching.
    pub env: HashMap<String, String>,
    /// Prepended to the Steam launch options string (Linux Proton path).
    pub steam_launch_prefix: Option<String>,
    /// Arguments appended to the game executable.
    pub game_args: Vec<String>,
    /// An optional wrapper script path (some loaders need a shell wrapper on Linux).
    pub wrapper: Option<PathBuf>,
}
```

### 5.3 Concrete Loaders

Each loader is a unit struct that implements `Loader`. They're cheap to construct and stateless.

**`BepInExLoader`**

- **Windows**: Doorstop is passive; no env/args needed once the profile root has `winhttp.dll` (or `doorstop_config.ini`). Launch as normal.
- **Linux (native)**: `LD_PRELOAD=./doorstop_libs/libdoorstop.so`, `DOORSTOP_ENABLED=1`, `DOORSTOP_TARGET_ASSEMBLY=BepInEx/core/BepInEx.Preloader.dll`.
- **Linux (Proton)**: `WINEDLLOVERRIDES="winhttp=n,b"`, `STEAM_COMPAT_MODS_PATHS=<profile_root>` prepended to the Steam launch command.
- Install rules: strip top-level folder, install `plugins/**` → `BepInEx/plugins/{pkg}/`, `patchers/**` → `BepInEx/patchers/{pkg}/`, `config/**` → `BepInEx/config/`. Core package (BepInExPack) maps everything to Root.

**`MelonLoaderLoader`**

- Injection is pre-installed into the game directory by the user or a bootstrapper.
- `--melonloader.loadmodsfromprofile <profile_root>` game arg.
- Install rules: `Mods/**` → `Mods/{pkg}/`.

**`UE4SSLoader`**

- Loaded by the game's shipping DLL via a proxy (`dwmapi.dll` / `xinput1_3.dll`).
- No special launch args needed after the proxy is in place.
- Install rules: `Mods/**` → `Mods/{pkg}/`.

**`GDWeaveLoader`**

- **Windows**: `--gdweave-root <profile_root>` game arg.
- Install rules: `mods/**` → `mods/{pkg}/`.

**`ShimloaderLoader`**, **`NorthstarLoader`**, **`LovelyLoader`**, etc. follow the same pattern and can be added incrementally.

---

## 6. Manifest Crate (`loadsmith-manifest`)

A Cargo-inspired dependency management layer. Decoupled from Thunderstore — it works with any registry source.

### 6.1 Manifest Format (TOML)

```toml
[profile]
name    = "my-profile"
game    = "lethal-company"
version = "1"

[dependencies]
# Source: thunderstore (default)
"BepInEx:BepInExPack"  = { version = "5.4.2100", source = "thunderstore" }
"Evaisa:LethalThings"  = { version = "*",         source = "thunderstore" }

# Source: github release
"Herpderp:SomeMod"    = { version = ">=1.2.0", source = "github", repo = "Herpderp/SomeMod" }

# Source: local file
"Local:MyMod"         = { path = "../my-mod" }
```

The `version` field accepts semver ranges. The `source` field selects the registry. Unknown fields in source definitions are passed through opaquely so registry-specific metadata survives round-trips.

### 6.2 Lockfile Format (TOML)

The lockfile pins every dependency (direct and transitive) to an exact version, URL, and checksum. It should be committed to version control.

```toml
lockfile-version = 1

[[package]]
namespace = "BepInEx"
name      = "BepInExPack"
version   = "5.4.2100"
source    = "thunderstore"
url       = "https://thunderstore.io/package/download/BepInEx/BepInExPack/5.4.2100/"
checksum  = "sha256:abc123..."
deps      = []

[[package]]
namespace = "Evaisa"
name      = "LethalThings"
version   = "0.10.6"
source    = "thunderstore"
url       = "https://..."
checksum  = "sha256:def456..."
deps      = ["BepInEx:BepInExPack@5.4.2100"]
```

### 6.3 Resolver

```rust
/// Resolve a manifest into a lockfile by querying registries.
/// Implements a simple depth-first SAT-free resolver (Thunderstore
/// doesn't have conflicting version ranges in practice; more complex
/// resolution can be added behind a feature flag later).
pub async fn resolve(
    manifest: &Manifest,
    registries: &dyn RegistrySet,
    existing_lock: Option<&Lockfile>,
) -> Result<Lockfile, ResolveError>
```

Locked versions are preserved if they still satisfy the manifest constraints, matching Cargo's behaviour.

---

## 7. Thunderstore Crate (`loadsmith-thunderstore`)

Self-contained integration with the Thunderstore platform.

### 7.1 API Client

```rust
pub struct ThunderstoreClient {
    http: reqwest::Client,
    base_url: Url,  // defaults to https://thunderstore.io
}

impl ThunderstoreClient {
    // Game/community definitions
    pub async fn list_communities(&self) -> Result<Vec<Community>>;
    pub async fn get_community(&self, id: &str) -> Result<Community>;

    // Package listings (uses the chunked v1 API for full package indexes)
    pub async fn list_packages(&self, community: &str) -> Result<Vec<ThunderstorePackage>>;
    pub async fn get_package(&self, community: &str, pkg: &PackageRef) -> Result<ThunderstorePackage>;

    // Download
    pub async fn download_package(&self, pkg: &PackageRef) -> Result<Bytes>;
}
```

The client implements `loadsmith_registry::Registry` so it slots into the manifest resolver automatically.

### 7.2 r2z Portable Profile

The r2z format is a zip file containing a `export.r2x` JSON manifest listing the profile's mods. It's used for sharing profiles between users and mod managers.

```rust
pub fn export_r2z(
    profile: &Profile,
    installed: &[InstalledManifest],
    name: &str,
) -> Result<Bytes, R2zError>

pub fn import_r2z(
    data: &[u8],
) -> Result<R2zManifest, R2zError>

#[derive(Serialize, Deserialize)]
pub struct R2zManifest {
    pub profile_name: String,
    pub mods: Vec<R2zMod>,
}

#[derive(Serialize, Deserialize)]
pub struct R2zMod {
    pub name: String,   // "Namespace-Name-Version"
    pub enabled: bool,
}
```

### 7.3 Thunderstore Package Types

```rust
#[derive(Serialize, Deserialize)]
pub struct ThunderstorePackage {
    pub namespace: String,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub versions: Vec<ThunderstoreVersion>,
    pub categories: Vec<String>,
    pub rating_score: u32,
    pub is_deprecated: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ThunderstoreVersion {
    pub version_number: Version,
    pub download_url: Url,
    pub dependencies: Vec<String>,  // "Namespace-Name-Version"
    pub file_size: u64,
}
```

---

## 8. Registry Crate (`loadsmith-registry`)

The abstraction layer that lets the manifest resolver speak to any package source.

### 8.1 Registry Trait

```rust
#[async_trait]
pub trait Registry: Send + Sync {
    /// Source identifier — matches the `source` field in manifest entries.
    fn id(&self) -> &str;

    /// Fetch available versions for a package.
    async fn fetch_versions(
        &self,
        id: &PackageId,
    ) -> Result<Vec<VersionInfo>, RegistryError>;

    /// Resolve a version specifier to an exact, downloadable reference.
    async fn resolve(
        &self,
        id: &PackageId,
        req: &VersionReq,
    ) -> Result<LockedPackage, RegistryError>;

    /// Download a locked package, returning the raw zip bytes.
    async fn download(
        &self,
        locked: &LockedPackage,
    ) -> Result<Bytes, RegistryError>;

    /// Fetch the direct dependencies of a specific version.
    async fn dependencies(
        &self,
        locked: &LockedPackage,
    ) -> Result<Vec<Dependency>, RegistryError>;
}
```

### 8.2 Built-in Registries

These are thin wrappers that implement `Registry`:

| Type | Notes |
|---|---|
| `ThunderstoreRegistry` | wraps `ThunderstoreClient`, available in the Thunderstore crate |
| `GitHubReleaseRegistry` | fetches from GitHub releases API, matches assets by glob |
| `LocalRegistry` | resolves `path = "..."` entries from disk |
| `CurseforgeRegistry` | stub — API key required, lower priority |
| `NexusRegistry` | stub — requires user auth |

### 8.3 RegistrySet

```rust
/// A collection of registries keyed by their source identifier.
pub struct RegistrySet {
    inner: HashMap<String, Box<dyn Registry>>,
}

impl RegistrySet {
    pub fn register(&mut self, registry: impl Registry + 'static);
    pub fn get(&self, id: &str) -> Option<&dyn Registry>;
}
```

---

## 9. Design Decisions

### Hard links for the package cache

Like gale, loadsmith should maintain a central package cache on disk. When installing a package into a profile, files are hard-linked from the cache rather than copied. This means:
- Installing the same package across 10 profiles costs only one copy of the bytes on disk.
- Installation is near-instant for cached packages.
- Removal is safe: deleting a hard link from the profile never touches the cache.

The `LinkStrategy` enum in `loadsmith-install` allows falling back to copy when the source and target are on different filesystems (hard links can't cross filesystem boundaries) or when the OS doesn't support them.

### No async in core or install

Core types and the install machinery are fully synchronous. Async lives only at the I/O boundary: downloading packages, querying APIs. This makes the install layer easy to test without an async runtime and avoids `Send` bound leakage into types that don't need it.

### Traits over enums for extensibility

Loader and Registry are traits, not enums. This means a consumer can implement their own loader or registry without forking loadsmith. The built-in implementations are in separate crates precisely so consumers can depend only on the core traits.

### Install rules as data

`InstallRule` structs are plain data rather than closures. This means they can be serialised (useful for a game definition file), logged, and unit-tested with table-driven tests. The pattern `{pkg}` substitution keeps the common case concise.

### Manifest and lockfile are format-stable

Once loadsmith hits 1.0, the `lockfile-version` field gates breaking format changes. Older lockfiles can always be re-read and re-resolved; newer fields are ignored gracefully. This mirrors Cargo's approach.

### Feature flags

```toml
[features]
default    = ["thunderstore"]
thunderstore = ["dep:reqwest", "dep:loadsmith-thunderstore"]
github     = ["dep:octocrab"]
# Consumers targeting embedded/WASM environments can disable network features.
```

---

## 10. Dependencies

| Crate | Why |
|---|---|
| `serde` + `serde_json` + `toml` | Serialisation for all formats |
| `semver` | Version types and range matching |
| `thiserror` | Error definition in library crates |
| `tokio` | Async runtime (only in async crates, behind a feature flag) |
| `reqwest` | HTTP client for API + downloads (feature-gated) |
| `zip` | Archive extraction |
| `globset` | Pattern matching for install rules |
| `camino` | UTF-8 typed paths (nice ergonomics, avoids OsString surprises) |
| `chrono` | Timestamps in `InstalledManifest` |
| `sha2` | Checksum verification |
| `url` | Typed URL in registry and API types |
| `async-trait` | Object-safe async traits until AFIT stabilises |
| `tracing` | Structured logging — consumers wire in the subscriber |

Avoid: `anyhow` in library code, `log` (use `tracing`), any GUI toolkit.

---

## 11. Phased Roadmap

### Phase 1 — Core + Install

- `loadsmith-core`: `PackageId`, `PackageRef`, `GameDef`, `Platform`, `Os`, `LoaderId`, error types.
- `loadsmith-install`: `InstallRule`, `InstallTarget`, `InstallContext`, `InstalledManifest`, `install`, `remove`, `list_files`, hard-link strategy.
- `BepInExLoader` in `loadsmith-loader` with full install rule coverage and launch args for Windows + Linux (native and Proton).
- Unit tests for install rules using in-memory zip fixtures.

### Phase 2 — Thunderstore Integration

- `loadsmith-thunderstore`: `ThunderstoreClient`, all relevant API types, `download_package`.
- `ThunderstoreRegistry` implementing the `Registry` trait.
- `export_r2z` / `import_r2z`.
- Integration test: fetch a real package, install it, verify files, remove it.

### Phase 3 — Manifest + Lockfile

- `loadsmith-manifest`: TOML format, `Manifest`, `Lockfile`, `resolve`.
- Wire `ThunderstoreRegistry` into the resolver.
- `LocalRegistry` for `path = "..."` deps.
- CLI example: `install` subcommand that reads a manifest, resolves, downloads, and installs.

### Phase 4 — Remaining Loaders

- `MelonLoaderLoader`, `UE4SSLoader`, `GDWeaveLoader`, `ShimloaderLoader`.
- `NorthstarLoader`, `LovelyLoader`, `ReturnOfModdingLoader`.
- Table-driven tests for all loaders' launch args across all OS/platform combinations.

### Phase 5 — Additional Registries

- `GitHubReleaseRegistry`.
- Stubs for Curseforge and Nexus (behind feature flags).
- Version resolver enhancements for non-exact version ranges.

### Phase 6 — Polish & Stabilisation

- Public API review and doc coverage.
- `loadsmith` facade crate with `prelude`.
- MSRV policy, semver guarantees, changelog.
- Publish to crates.io.

---

## 12. Repository Layout (Annotated)

```
loadsmith/
├── Cargo.toml
│   └── [workspace] members = ["crates/*"]
│
├── crates/
│   │
│   ├── loadsmith-core/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── package.rs      # PackageId, PackageRef, Version re-export
│   │       ├── game.rs         # GameDef, Platform, Os
│   │       ├── loader_id.rs    # LoaderId newtype
│   │       └── error.rs        # top-level Error enum
│   │
│   ├── loadsmith-install/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rule.rs         # InstallRule, InstallTarget, GlobPattern
│   │       ├── install.rs      # install(), LinkStrategy
│   │       ├── remove.rs       # remove()
│   │       ├── list.rs         # list_files()
│   │       └── manifest.rs     # InstalledManifest
│   │
│   ├── loadsmith-loader/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs       # Loader trait, LaunchArgs
│   │       ├── bepinex.rs
│   │       ├── melonloader.rs
│   │       ├── ue4ss.rs
│   │       ├── gdweave.rs
│   │       └── shimloader.rs
│   │
│   ├── loadsmith-manifest/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manifest.rs     # Manifest, Dependency, VersionReq
│   │       ├── lockfile.rs     # Lockfile, LockedPackage
│   │       └── resolve.rs      # resolve()
│   │
│   ├── loadsmith-registry/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs     # Registry trait, RegistrySet
│   │       ├── version_info.rs # VersionInfo, LockedPackage
│   │       └── local.rs        # LocalRegistry
│   │
│   ├── loadsmith-thunderstore/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs       # ThunderstoreClient
│   │       ├── types.rs        # ThunderstorePackage, Community, ...
│   │       ├── registry.rs     # ThunderstoreRegistry: Registry
│   │       └── r2z.rs          # export_r2z, import_r2z
│   │
│   └── loadsmith/              # facade
│       └── src/
│           ├── lib.rs
│           └── prelude.rs
│
└── examples/
    ├── install-mod/src/main.rs
    └── thunderstore-fetch/src/main.rs
```
