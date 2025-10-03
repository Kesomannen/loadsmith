use loadsmith::GDWeave;

mod common;

fn make_loader() -> GDWeave {
    GDWeave::new()
}

#[test]
fn it_extracts_installs_and_uninstalls_package() -> anyhow::Result<()> {
    common::test_mod_operations(
        make_loader(),
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
