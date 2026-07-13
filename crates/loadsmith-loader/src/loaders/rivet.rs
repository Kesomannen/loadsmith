use std::{path::PathBuf, sync::LazyLock};

use loadsmith_core::PackageRef;
use loadsmith_install::{InstallRule, InstallRuleset};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

#[derive(Debug, Clone)]
pub struct Rivet;

impl Rivet {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Rivet {
    fn default() -> Self {
        Self::new()
    }
}

impl Loader for Rivet {
    fn id(&self) -> &'static str {
        "Rivet"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            // extract all non-top-level files into the package root
            vec![glob_rule!("*/*" => ".").strip_top_level(true).into()]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("*" => "Rivet/Mods")
                    .with_subdir(true)
                    .use_links(true)
                    .into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let rivet_directory = ctx.format_proton_path(ctx.profile_path().join("Rivet"));
        let rivet_target =
            ctx.format_proton_path(ctx.profile_path().join("Rivet").join("Loader.dll"));

        let args = LaunchArgs::new()
            .arg("-rivetEnable")
            .arg("true")
            .arg("-rivetTarget")
            .arg(rivet_target)
            .arg("-rivetDirectory")
            .arg(rivet_directory);

        Ok(args)
    }

    fn package_dir(&self, package: &PackageRef) -> Option<PathBuf> {
        Some(PathBuf::from("Rivet/Mods").join(package.id.as_str()))
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn log_file(&self) -> Option<PathBuf> {
        Some("Rivet/Rivet.log".into())
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        Some("version.dll".into())
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageRef;

    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            Rivet::new(),
            PackageRef::new("ReDoIngMods-Rivet".to_string(), (0, 1, 9)),
            true,
        ), [
            "README.md" => None,
            "RivetPack/version.dll" => "./version.dll",
            "RivetPack/Rivet.ini" => "./Rivet.ini",
            "OtherFolder/file.txt" => "./file.txt",
            "RivetPack/Rivet/Loader.dll" => "./Rivet/Loader.dll",
        ]);
    }

    #[test]
    fn map_package_files() {
        assert_maps!(
            MapFileTester::new(
                Rivet::new(),
                PackageRef::new("Author-Name".to_string(), (1, 0, 0)),
                false,
            ),
            [
                "manifest.json" => "Rivet/Mods/Author-Name/manifest.json",
                "nested/file.txt" => "Rivet/Mods/Author-Name/nested/file.txt",
                "1/2/file.txt" => "Rivet/Mods/Author-Name/1/2/file.txt",
            ]
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = Rivet::new();
        let package = PackageRef::new("Author-Name".to_string(), (1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(package_dir, Some(PathBuf::from("Rivet/Mods/Author-Name")));
    }
}
