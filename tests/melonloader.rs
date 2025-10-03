use loadsmith::{MelonLoader, MelonLoaderBuilder};

mod common;

fn make_loader() -> MelonLoader {
    MelonLoaderBuilder::new().build()
}

#[test]
fn it_extracts_installs_and_uninstalls_package() -> anyhow::Result<()> {
    common::test_mod_operations(
        &make_loader(),
        "melonloader_1",
        "modname",
        vec![
            "Mods/file1.dll",
            "Mods/file2.txt",
            "UserLibs/file3.txt",
            "UserLibs/file4.lib.dll",
            "MelonLoader/Managed/file5.txt",
            "MelonLoader/Managed/file6.managed.dll",
            "UserData/ModManager/modname/file7.txt",
            "UserData/ModManager/modname/nested/file8.txt",
            "UserData/ModManager/modname/nested/file9.txt",
            "MelonLoader/file10.txt",
            "MelonLoader/Libs/file11.txt",
        ],
        Vec::new(),
        vec!["Mods/icon.png", "Mods/ignored.txt"],
    )
}
