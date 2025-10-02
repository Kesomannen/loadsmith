use loadsmith::{MelonLoader, MelonLoaderBuilder, ModLoader};
use tempfile::TempDir;

mod common;

fn make_loader() -> MelonLoader {
    MelonLoaderBuilder::new().build()
}

#[test]
fn it_installs_loader() -> anyhow::Result<()> {
    let tempdir = TempDir::new_in(format!("{}/temp", env!("CARGO_MANIFEST_DIR")))?;
    let melonloader = make_loader();

    melonloader.loader_installer().extract_and_install(
        common::open_zip("melonloader"),
        "melonloader",
        tempdir.path(),
    )?;

    melonloader
        .loader_installer()
        .uninstall(tempdir.path(), "melonloader")?;

    let _ = tempdir.keep();

    Ok(())
}
