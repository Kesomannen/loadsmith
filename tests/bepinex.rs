use std::{collections::HashSet, path::PathBuf};

use loadsmith::{BepInEx, BepInExBuilder, PackageInstaller, rule::Rule};
use tempdir::TempDir;

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
fn it_extracts_and_installs() -> anyhow::Result<()> {
    let tempdir = TempDir::new("loadsmith")?;
    let bepinex = make_loader();

    bepinex.extract_and_install(common::open_zip("bepinex_1"), "modname", tempdir.path())?;

    let files = vec![
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
    ];

    for file in &files {
        assert!(
            tempdir.path().join(file).exists(),
            "file at {file} wasn't installed!",
        );
    }

    let expected_listed_files = files
        .into_iter()
        .filter(|file| !file.starts_with("BepInEx/config")) // we don't expect to track the config
        .map(|file| tempdir.path().join(file))
        .collect::<HashSet<_>>();

    let actual_installed_files = bepinex
        .package_files(tempdir.path(), "modname")?
        .into_iter()
        .collect::<HashSet<_>>();

    assert_eq!(actual_installed_files, expected_listed_files);

    Ok(())
}
