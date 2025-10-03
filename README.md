# Loadsmith

A crate for handling filesystem operations with a range of mod loaders.

Loadsmith can help you with:

- Unpacking mods according to packaging rules
- Keeping track of mod files and doing mod uninstallation
- Creating launch arguments to invoke the respective mod loader

This crate is primarily focused on [Thunderstore](https://thunderstore.io) compatibility, and was originally a part of the mod manager [Gale](https://github.com/Kesomannen/gale).
The supported mod loaders is those found on Thunderstore's website, including:

- BepInEx
- MelonLoader
- Shimloader
- ReturnOfModding
- GDWeave
- Lovely
- Northstar
- BepisLoader

## Usage

The main two traits are `ModLoader` and `PackageInstaller`.

A `ModLoader` holds methods related to one modding framework and sometimes pieces of game-specific configuration.

A `PackageInstaller` is responsible for extracting, installing and bookkeeping the installed mods in a directory. Each `ModLoader` has a pair of installers: `package_installer()` and `loader_installer()`. The former used to install regular packages/mods, and the later for the mod loader itself. 

> [!TIP]
> `ModLoader` implements `PackageInstaller` itself by forwarding calls to `package_installer()`.

See more in the respective traits' docs.

### Install a mod

Installation happens in two steps:

- Firstly, the mod's zip archive is extracted according to the mod loader's packaging rules.
- Secondly, the contents of the extracted directory are copied/linked to the destination directory (called the mod profile). Here the installer may do some extra operations, for example writing to a [state file].

This can be used to easily and efficiently cache mods, as you can extract the archive to some directory, then install it into as many profiles you want. If you want a simpler flow however, there is also [`PackageInstaller::extract_and_install`], which extracts to a temporary directory.

**Example**

```rust
use std::path::Path;
use loadsmith::loaders::BepInEx;

// create a BepInEx loader with the default config
let loader = BepInEx::new();
// where our mods will end up
let profile_path = Path::new("./profiles/Default");
// our mod archive, maybe downloaded from Thunderstore
let zip_path = Path::new("./downloads/notnotnotswipez-MoreCompany-1.10.0.zip");

let zip = loadsmith::open_zip(zip_path)?;
loader.extract_and_install(zip, "notnotnotswipez-MoreCompany", profile_path)?;
```
