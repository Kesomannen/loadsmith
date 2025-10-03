use loadsmith::{ModLoader, loaders::GDWeave};

mod common;

fn make_loader() -> GDWeave {
    GDWeave::new()
}

#[test]
fn it_extracts_installs_and_uninstalls_package() -> anyhow::Result<()> {
    common::test_mod_operations(
        &make_loader(),
        "gdweave_1",
        "modname",
        vec![
            "GDWeave/mods/modname/manifest.json",
            "GDWeave/mods/modname/file1.txt",
            "GDWeave/mods/modname/nested/file2.txt",
        ],
        Vec::new(),
        vec![
            "GDWeave/mods/modname/file3.txt",
            "GDWeave/mods/modname/non_root/file3.txt",
            "GDWeave/mods/manifest.json",
        ],
    )
}

#[test]
fn it_extracts_installs_and_uninstalls_gdweave() -> anyhow::Result<()> {
    common::test_mod_operations(
        make_loader().loader_installer(),
        "gdweave",
        "GDWeave",
        vec![
            "winmm.dll",
            "GDWeave/core/FASM.DLL",
            "GDWeave/core/FASM-LICENSE.TXT",
            "GDWeave/core/FASMX64.DLL",
            "GDWeave/core/GDWeave.deps.json",
            "GDWeave/core/GDWeave.dll",
            "GDWeave/core/GDWeave.pdb",
            "GDWeave/core/GDWeave.runtimeconfig.json",
            "GDWeave/core/Iced.dll",
            "GDWeave/core/Reloaded.Assembler.dll",
            "GDWeave/core/Reloaded.Assembler.targets",
            "GDWeave/core/Reloaded.Hooks.Definitions.dll",
            "GDWeave/core/Reloaded.Hooks.dll",
            "GDWeave/core/Reloaded.Memory.Buffers.dll",
            "GDWeave/core/Reloaded.Memory.dll",
            "GDWeave/core/Reloaded.Memory.Sigscan.Definitions.dll",
            "GDWeave/core/Reloaded.Memory.Sigscan.dll",
            "GDWeave/core/Serilog.dll",
            "GDWeave/core/Serilog.Sinks.Console.dll",
            "GDWeave/core/Serilog.Sinks.File.dll",
        ],
        Vec::new(),
        Vec::new(),
    )
}
