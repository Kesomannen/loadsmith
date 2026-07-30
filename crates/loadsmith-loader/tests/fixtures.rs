use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use loadsmith_core::PackageRef;
use loadsmith_install::InstallRuleset;
use loadsmith_loader::{
    BepInEx, BepisLoader, GDWeave, Loader, Lovely, MelonLoader, Northstar, ReturnOfModding, Rivet,
    Shimloader,
};

fn test_fixture_category(category: &str, rules: &InstallRuleset) {
    let category_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(category);

    for entry in std::fs::read_dir(category_path).expect("failed to read fixture directory") {
        let entry = entry.expect("failed to read fixture directory entry");
        let path = entry.path();
        let file_stem = path
            .file_stem()
            .expect("failed to get file stem")
            .to_str()
            .expect("file stem should be UTF-8");

        let package = file_stem
            .parse()
            .expect("failed to parse package reference from file stem");

        test_fixture(&path, &package, rules);
    }
}

fn test_fixture(path: &Path, package: &PackageRef, rules: &InstallRuleset) {
    let contents = std::fs::read_to_string(&path).expect("failed to read fixture file");
    let lines = contents
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty());

    let mapped = lines
        .map(|line| {
            let path = rules
                .map_file(line, package)
                .map(|path| path.into_string().replace('\\', "/"));
            (line, path)
        })
        .collect::<BTreeMap<_, _>>();

    insta::assert_yaml_snapshot!(package.to_string(), mapped)
}

macro_rules! fixture_test {
    ($name:ident, $category:expr, $rules:expr) => {
        #[test]
        fn $name() {
            test_fixture_category($category, $rules);
        }
    };
}

fixture_test!(
    bep_in_ex_loader,
    "BepInEx/loader",
    &BepInEx::with_default_rules().loader_install_rules()
);

fixture_test!(
    bep_in_ex_package,
    "BepInEx/package",
    &BepInEx::with_default_rules().package_install_rules()
);

fixture_test!(
    melon_loader_loader,
    "MelonLoader/loader",
    &MelonLoader::with_default_legacy_rules().loader_install_rules()
);

fixture_test!(
    melon_loader_package_legacy,
    "MelonLoader/package_legacy",
    &MelonLoader::with_default_legacy_rules().package_install_rules()
);

fixture_test!(
    melon_loader_package_recursive,
    "MelonLoader/package_recursive",
    &MelonLoader::with_default_recursive_rules().package_install_rules()
);

fixture_test!(
    shimloader_loader,
    "Shimloader/loader",
    &Shimloader::with_default_rules().loader_install_rules()
);

fixture_test!(
    shimloader_package,
    "Shimloader/package",
    &Shimloader::with_default_rules().package_install_rules()
);

fixture_test!(
    return_of_modding_loader,
    "ReturnOfModding/loader",
    &ReturnOfModding::with_default_rules().loader_install_rules()
);

fixture_test!(
    return_of_modding_package,
    "ReturnOfModding/package",
    &ReturnOfModding::with_default_rules().package_install_rules()
);

fixture_test!(
    bepis_loader_loader,
    "BepisLoader/loader",
    &BepisLoader::with_default_rules().loader_install_rules()
);

fixture_test!(
    bepis_loader_package,
    "BepisLoader/package",
    &BepisLoader::with_default_rules().package_install_rules()
);

fixture_test!(
    gd_weave_loader,
    "GDWeave/loader",
    &GDWeave::new().loader_install_rules()
);

fixture_test!(
    gd_weave_package,
    "GDWeave/package",
    &GDWeave::new().package_install_rules()
);

fixture_test!(
    lovely_loader,
    "Lovely/loader",
    &Lovely::new().loader_install_rules()
);

fixture_test!(
    lovely_package,
    "Lovely/package",
    &Lovely::new().package_install_rules()
);

fixture_test!(
    rivet_loader,
    "Rivet/loader",
    &Rivet::new().loader_install_rules()
);

fixture_test!(
    rivet_package,
    "Rivet/package",
    &Rivet::new().package_install_rules()
);

fixture_test!(
    northstar_loader,
    "Northstar/loader",
    &Northstar::with_default_rules().loader_install_rules()
);

fixture_test!(
    northstar_package,
    "Northstar/package",
    &Northstar::with_default_rules().package_install_rules()
);
