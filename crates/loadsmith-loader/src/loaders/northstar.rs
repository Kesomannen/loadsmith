use std::{path::PathBuf, sync::LazyLock};

use globset::{GlobBuilder, GlobSet};
use loadsmith_install::rule::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob, glob_rule};

/// Loader implementation for [Northstar](https://northstar.tf/), a mod loader and custom server
/// framework for Titanfall 2.
///
/// Northstar packages are installed into `R2Northstar/mods`, and the game is launched with the
/// `-northstar` and `-profile=<path>` arguments. Files that are only relevant to Thunderstore
/// (`manifest.json`, `README.md`, `icon.png`, and `LICENSE`) are excluded from installation.
///
/// The package installation rules for Northstar are configurable, with default rules matching
/// the Thunderstore package format for Northstar mods. You'll find a list of the rules in the
/// documentation for [`with_default_rules`](Northstar::with_default_rules).
#[derive(Debug, Clone)]
pub struct Northstar {
    package_install_ruleset: OwnedInstallRuleset,
}

impl Northstar {
    /// Creates a `Northstar` loader with a custom install ruleset.
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    /// Creates a `Northstar` loader with the default install rules.
    ///
    /// |                  | Subdir | Flattened | Mutable | Extension | Default |
    /// |------------------|--------|-----------|---------|-----------|---------|
    /// | R2Northstar/mods | `-`    | `-`       | `-`     | `-`       | `-`     |
    ///
    /// The files `manifest.json`, `README.md`, `icon.png`, and `LICENSE` are always excluded,
    /// and no rule is configured as the default, meaning that files that do not match any rule
    /// are not installed.
    ///
    /// Read more about how rules work in the documentation of the [`InstallRule`] struct.
    pub fn with_default_rules() -> Self {
        let exclude = GlobSet::builder()
            .add(glob!("manifest.json"))
            .add(glob!("README.md"))
            .add(glob!("icon.png"))
            .add(glob!("LICENSE"))
            .build()
            .expect("constant globs should be valid");

        let ruleset = OwnedInstallRuleset::from_rule_iter(
            vec![
                RouteRule::new_static("R2Northstar/mods")
                    .with_subdir(false)
                    .with_flatten(false),
            ],
            None,
        )
        .expect("rules are not empty so there should always be a valid default rule index")
        .with_exclude(exclude);

        Self::with_rules(ruleset)
    }

    /// Adds a rule to the end of the package install ruleset.
    pub fn add_install_rule(&mut self, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.add_rule(rule.into());
    }

    /// Inserts a rule at the given index of the package install ruleset.
    pub fn insert_install_rule(&mut self, index: usize, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.insert_rule(index, rule.into());
    }
}

impl Default for Northstar {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl Loader for Northstar {
    fn id(&self) -> &'static str {
        "Northstar"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("Northstar/*" => ".")
                    .strip_top_level(true)
                    .with_links(true)
                    .into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.package_install_ruleset.as_ref()
    }

    fn prepare_launch(&self, ctx: &LaunchContext) -> Result<()> {
        static GLOB_SET: LazyLock<GlobSet> = LazyLock::new(|| {
            GlobSet::builder()
                .add(
                    GlobBuilder::new("*.{dll,exe,bat}")
                        .literal_separator(true)
                        .build()
                        .expect("constant glob should be valid"),
                )
                .build()
                .expect("constant globs should be valid")
        });

        ctx.copy_glob_to_game(&GLOB_SET)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let r2northstar_path = ctx.profile_path().join("R2Northstar");
        let path = ctx.format_proton_path(&r2northstar_path);

        let args = LaunchArgs::new()
            .arg("-northstar")
            .arg(format!("-profile={path}"));

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
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
            Northstar::with_default_rules(),
            PackageRef::new("northstar-Northstar".to_string(), Version::new(1, 31, 10)),
            true,
        ), [
            "README.md" => None,
            "Northstar/r2ds.bat" => "./r2ds.bat",
            "Northstar/R2Northstar/file.txt" => "./R2Northstar/file.txt",
            "Northstar/R2Northstar/plugins/DiscordRPC.dll" => "./R2Northstar/plugins/DiscordRPC.dll",
        ])
    }

    #[test]
    fn map_package_files() {
        assert_maps!(MapFileTester::new(
            Northstar::with_default_rules(),
            PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0)),
            false,
        ), [
            "manifest.json" => None,
            "README.md" => None,
            "icon.png" => None,
            "LICENSE" => None,
            "file.txt" => None,
            "mods/file.txt" => "R2Northstar/mods/file.txt",
            "mods/nested/file.txt" => "R2Northstar/mods/nested/file.txt",
            "nested/mods/file.txt" => "R2Northstar/mods/nested/file.txt",
        ]);
    }

    #[test]
    fn package_dir_works() {
        let loader = Northstar::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0));

        assert!(loader.package_dir(&package).is_none());
    }
}
