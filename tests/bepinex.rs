use std::path::PathBuf;

use loadsmith::{
    ModLoader,
    loaders::{BepInEx, BepInExBuilder},
    rule::Rule,
};

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
        &make_loader(),
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
    common::test_mod_operations(
        make_loader().loader_installer(),
        "bepinex",
        "BepInEx",
        vec![
            "doorstop_config.ini",
            "start_game_bepinex.sh",
            "version.dll",
            "winhttp.dll",
            "doorstop_libs/libdoorstop_x64.dylib",
            "doorstop_libs/libdoorstop_x64.so",
            "doorstop_libs/libdoorstop_x86.so",
            "BepInEx/config/BepInEx.cfg",
            "BepInEx/core/0Harmony.dll",
            "BepInEx/core/0Harmony.xml",
            "BepInEx/core/0Harmony20.dll",
            "BepInEx/core/BepInEx.dll",
            "BepInEx/core/BepInEx.xml",
            "BepInEx/core/BepInEx.Harmony.dll",
            "BepInEx/core/BepInEx.Harmony.xml",
            "BepInEx/core/BepInEx.Preloader.dll",
            "BepInEx/core/BepInEx.Preloader.xml",
            "BepInEx/core/HarmonyXInterop.dll",
            "BepInEx/core/Mono.Cecil.dll",
            "BepInEx/core/Mono.Cecil.Mdb.dll",
            "BepInEx/core/Mono.Cecil.Pdb.dll",
            "BepInEx/core/Mono.Cecil.Rocks.dll",
            "BepInEx/core/MonoMod.dll",
            "BepInEx/core/MonoMod.RuntimeDetour.dll",
            "BepInEx/core/MonoMod.RuntimeDetour.xml",
            "BepInEx/core/MonoMod.Utils.dll",
            "BepInEx/core/MonoMod.Utils.xml",
            "BepInEx/patchers/BepInEx.MonoMod.Loader.dll",
        ],
        Vec::new(),
        Vec::new(),
    )
}
