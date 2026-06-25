use std::{collections::BTreeMap, path::PathBuf};

use loadsmith_core::PackageRef;
use loadsmith_install::InstallRuleset;
use loadsmith_loader::{BepInExLoader, Loader};

fn test_fixture_category(category: &str, rules: &InstallRuleset) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(category);

    for entry in std::fs::read_dir(path).expect("failed to read fixture directory") {
        let entry = entry.expect("failed to read fixture directory entry");
        let path = entry.path();
        let file_stem = path
            .file_stem()
            .expect("failed to get file stem")
            .to_str()
            .expect("file stem should be UTF-8");

        let Some((id, version)) = file_stem.rsplit_once('-') else {
            panic!("invalid fixture file name: {file_stem}");
        };

        let package_id = PackageRef::new(id.to_string(), version);
        test_fixture(category, &package_id, rules);
    }
}

fn test_fixture(category: &str, package: &PackageRef, rules: &InstallRuleset) {
    let file_name = format!("{}-{}.txt", package.id.0, package.version);

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(category)
        .join(file_name);

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
fn bepinex_loader_fixtures() {
    test_fixture_category(
        "BepInEx/loader",
        &BepInExLoader::with_default_rules().loader_install_rules(),
    );
}

#[test]
fn bepinex_package_fixtures() {
    test_fixture_category(
        "BepInEx/package",
        &BepInExLoader::with_default_rules().package_install_rules(),
    );
}
