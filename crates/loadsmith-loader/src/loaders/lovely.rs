use std::{path::PathBuf, sync::LazyLock};

use loadsmith_core::PackageRef;
use loadsmith_install::{InstallRule, InstallRuleset};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

#[derive(Debug, Clone)]
pub struct Lovely;

impl Lovely {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Lovely {
    fn default() -> Self {
        Self::new()
    }
}

impl Loader for Lovely {
    fn id(&self) -> &'static str {
        "Lovely"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("*.dll" => ".").use_links(true).into(),
                glob_rule!("lovely/*" => "mods/lovely")
                    .strip_top_level(true)
                    .into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("*" => "mods")
                    .with_subdir(true)
                    .use_links(true)
                    .into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let mods_path = ctx.profile_path().join("mods");
        let path = ctx.format_proton_path(&mods_path);

        let args = LaunchArgs::new().arg("--mod-dir").arg(&*path);

        Ok(args)
    }

    fn package_dir(&self, package: &PackageRef) -> Option<PathBuf> {
        Some(PathBuf::from("mods").join(package.id().as_str()))
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn log_file(&self) -> Option<PathBuf> {
        Some("mods/lovely/log".into())
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
            Lovely::new(),
            PackageRef::new("Thunderstore-lovely".to_string(), (0, 9, 0)),
            true,
        ), [
            "README.md" => None,
            "version.dll" => "./version.dll",
            "lovely/config.toml" => "mods/lovely/config.toml",
        ]);
    }

    #[test]
    fn map_package_files() {
        assert_maps!(
            MapFileTester::new(
                Lovely::new(),
                PackageRef::new("Author-Name".to_string(), (1, 0, 0)),
                false,
            ),
            [
                "manifest.json" => "mods/Author-Name/manifest.json",
                "nested/file.txt" => "mods/Author-Name/nested/file.txt",
                "1/2/file.txt" => "mods/Author-Name/1/2/file.txt",
            ]
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = Lovely::new();
        let package = PackageRef::new("Author-Name".to_string(), (1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(package_dir, Some(PathBuf::from("mods/Author-Name")));
    }
}
