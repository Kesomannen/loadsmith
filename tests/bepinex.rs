use std::path::PathBuf;

use loadsmith::{BepInEx, BepInExBuilder, ModLoader, PackageInstaller, rule::Rule};
use tempfile::TempDir;

mod common;

fn make_loader() -> BepInEx {
    BepInExBuilder::new()
        .with_extra_rules(vec![
            Rule::separated("custom_1", PathBuf::from("BepInEx/custom_1")),
            Rule::tracked("custom_2", PathBuf::from("BepInEx/custom_2")),
        ])
        .build()
}

#[test]
fn it_extracts_installs_and_uninstalls_package() -> anyhow::Result<()> {
    common::test_mod_operations(
        make_loader(),
        "bepinex_1",
        "modname",
        vec![
            "BepInEx/plugins/modname/file1.txt",
            "BepInEx/plugins/modname/file2.txt",
            "BepInEx/plugins/modname/file3.txt",
            "BepInEx/plugins/modname/file4.txt",
            "BepInEx/plugins/modname/nested/file5.txt",
            "BepInEx/patchers/modname/file6.txt",
            "BepInEx/core/modname/file7.txt",
            "BepInEx/config/file8.txt",
            "BepInEx/config/nested/file9.txt",
            "BepInEx/monomod/modname/file10.txt",
            "BepInEx/monomod/modname/file11.mm.dll",
            "BepInEx/plugins/modname/file12.mm.dll",
            "BepInEx/custom_1/modname/file13.txt",
            "BepInEx/custom_1/modname/nested/file14.txt",
            "BepInEx/custom_1/modname/nested/double_nest/file15.txt",
            "BepInEx/custom_1/modname/nested/file16.txt",
            "BepInEx/custom_2/file17.txt",
            "BepInEx/custom_2/nested/file18.txt",
        ],
        vec![
            "BepInEx/config/file8.txt",
            "BepInEx/config/nested/file9.txt",
        ],
        Vec::new(),
    )
}

#[test]
fn it_installs_loader() -> anyhow::Result<()> {
    let tempdir = TempDir::new_in(format!("{}/temp", env!("CARGO_MANIFEST_DIR")))?;
    let zip = common::open_zip("bepinex");

    let loader = make_loader();

    loader
        .loader_installer()
        .extract_and_install(zip, "BepInEx", tempdir.path())?;

    let zip = common::open_zip("bepinex_1");
    loader.extract_and_install(zip, "modname", tempdir.path())?;

    loader
        .loader_installer()
        .uninstall(tempdir.path(), "BepInEx")?;

    let _ = tempdir.keep();

    Ok(())
}
