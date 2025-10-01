# Loadsmith

A crate for handling mod unpacking, installation and launch for a range of mod loaders, focused on [Thunderstore](https://thunderstore.io) compatibility

This code was originally a part of [Gale](https://github.com/Kesomannen/gale), but is now available as a standalone crate.

## Supported mod loaders

- BepInEx
- MelonLoader
- Shimloader
- ReturnOfModding
- GDWeave
- Lovely

## Example

```rust
use std::{process::Command, path::Path};

let bepinex = loadsmith::BepInEx::new();
let profile_path = Path::new("./profiles/Default");
let game_path = Path::new("C:/Program Files/Steam/steamapps/common/Lethal Company");

// Install a mod from a thunderstore-structured zip
let zip = loadsmith::open_zip("notnotnotswipez-MoreCompany.zip")?; // or use the `zip` crate directly
bepinex.extract_and_install(zip, "notnotnotswipez-MoreCompany", profile_path)?;

// Launch the modded game via Steam
bepinex.prepare_launch(profile_path, game_path)?;
Command::new("steam")
    .args(["-applaunch", "1966720"])
    .args(bepinex.get_launch_args(profile_path)?)
    .spawn()?;
```