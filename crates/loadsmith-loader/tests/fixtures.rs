use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use loadsmith_core::PackageRef;
use loadsmith_install::InstallRuleset;
use loadsmith_loader::{BepInEx, Loader, MelonLoader};

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
        .map(|line| (line, rules.map_file(line, package)))
        .collect::<BTreeMap<_, _>>();

    insta::assert_yaml_snapshot!(package.to_string(), mapped)
}

#[test]
fn bep_in_ex_loader() {
    test_fixture_category(
        "BepInEx/loader",
        &BepInEx::with_default_rules().loader_install_rules(),
    );
}

#[test]
fn bep_in_ex_package() {
    test_fixture_category(
        "BepInEx/package",
        &BepInEx::with_default_rules().package_install_rules(),
    );
}

#[test]
fn melon_loader_loader() {
    test_fixture_category(
        "MelonLoader/loader",
        &MelonLoader::with_default_legacy_rules().loader_install_rules(),
    );
}

#[test]
fn melon_loader_package_legacy() {
    test_fixture_category(
        "MelonLoader/package_legacy",
        &MelonLoader::with_default_legacy_rules().package_install_rules(),
    );
}

#[test]
fn melon_loader_package_recursive() {
    test_fixture_category(
        "MelonLoader/package_recursive",
        &MelonLoader::with_default_recursive_rules().package_install_rules(),
    );
}
