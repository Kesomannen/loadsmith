use std::{path::PathBuf, sync::LazyLock};

use loadsmith_install::rule::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

/// Loader implementation for [Return of Modding](https://github.com/return-of-modding/ReturnOfModding),
/// a Lua mod loader for Risk of Rain Returns and Hades 2.
///
/// Packages are routed into `ReturnOfModding/plugins`, `ReturnOfModding/plugins_data`, and
/// `ReturnOfModding/config` (mutable), and the profile directory is passed to the game as the
/// `--rom_modding_root_folder` launch argument.
///
/// The package installation rules for Return of Modding are configurable, with default rules
/// matching the Thunderstore package format for Return of Modding mods. You'll find a list of
/// the rules in the documentation for
/// [`with_default_rules`](ReturnOfModding::with_default_rules).
#[derive(Debug, Clone)]
pub struct ReturnOfModding {
    package_install_ruleset: OwnedInstallRuleset,
}

impl ReturnOfModding {
    /// Creates a `ReturnOfModding` loader with a custom install ruleset.
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    /// Creates a `ReturnOfModding` loader with the default install rules.
    ///
    /// |                              | Subdir | Flattened | Mutable | Extension | Default |
    /// |------------------------------|--------|-----------|---------|-----------|---------|
    /// | ReturnOfModding/plugins      | `X`    | `-`       | `-`     | `-`       | `X`     |
    /// | ReturnOfModding/plugins_data | `X`    | `-`       | `-`     | `-`       | `-`     |
    /// | ReturnOfModding/config       | `X`    | `-`       | `X`     | `-`       | `-`     |
    ///
    /// Read more about how rules work in the documentation of the [`InstallRule`] struct.
    pub fn with_default_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            [
                RouteRule::new_static("ReturnOfModding/plugins").with_flatten(false),
                RouteRule::new_static("ReturnOfModding/plugins_data").with_flatten(false),
                RouteRule::new_static("ReturnOfModding/config")
                    .with_flatten(false)
                    .with_mutable(true),
            ],
            Some(0),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }
}

impl Loader for ReturnOfModding {
    fn id(&self) -> &'static str {
        "Return Of Modding"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                // extract all non-top-level files into the package root
                glob_rule!("*/**" => ".").strip_top_level(true).into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.package_install_ruleset.as_ref()
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let path = ctx.format_proton_path(ctx.profile_path());

        let args = LaunchArgs::new()
            .arg("--rom_modding_root_folder")
            .arg(&*path);

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        vec!["ReturnOfModding/config".into()]
    }

    fn log_file(&self) -> Option<PathBuf> {
        None
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::{PackageRef, Version};

    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            ReturnOfModding::with_default_rules(),
            PackageRef::new("Hell2Modding-Hell2Modding".to_string(), Version::new(1, 0, 105)),
            true,
        ), [
            "icon.png" => None,
            "README.md" => None,
            "ReturnOfModdingPack/d3d12.dll" => "./d3d12.dll",
            "ReturnOfModdingPack/version.dll" => "./version.dll",
            "Other/file.txt" => "./file.txt",
        ]);
    }

    #[test]
    fn map_package_files() {
        assert_maps!(
            MapFileTester::new(
                ReturnOfModding::with_default_rules(),
                PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0)),
                false,
            ),
            [
                "README.md" => "ReturnOfModding/plugins/Author-Name/README.md",
                "nested/file.txt" => "ReturnOfModding/plugins/Author-Name/nested/file.txt",
                "plugins/nested/file.txt" => "ReturnOfModding/plugins/Author-Name/nested/file.txt",
                "config/settings.json" => "ReturnOfModding/config/Author-Name/settings.json",
                "plugins_data/file.txt" => "ReturnOfModding/plugins_data/Author-Name/file.txt",
                "plugins_data/nested/file.txt" => "ReturnOfModding/plugins_data/Author-Name/nested/file.txt",
                "nested/plugins_data/file.txt" => "ReturnOfModding/plugins_data/Author-Name/nested/file.txt",
            ]
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = ReturnOfModding::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(
            package_dir,
            Some(PathBuf::from("ReturnOfModding/plugins/Author-Name"))
        );
    }
}
