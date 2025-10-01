use super::*;
use std::path::PathBuf;

#[test]
fn it_extracts_correctly() -> anyhow::Result<()> {
    let zip_path: PathBuf = format!(
        "{}/files/FlipMods-ReservedFlashlightSlot-2.0.10.zip",
        env!("CARGO_MANIFEST_DIR")
    )
    .into();

    let profile_path: PathBuf = format!("{}/temp/profile", env!("CARGO_MANIFEST_DIR")).into();

    let bepinex = BepInEx::new();

    bepinex.extract_and_install(
        open_zip(zip_path)?,
        "FlipMods-ReservedFlashlightSlot",
        &profile_path,
    )?;

    Ok(())
}
