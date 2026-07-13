use std::{path::PathBuf, sync::LazyLock};

use globset::{GlobBuilder, GlobSet};
use loadsmith_install::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

#[derive(Debug, Clone)]
pub struct Northstar {
    package_install_ruleset: OwnedInstallRuleset,
}

impl Northstar {
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    pub fn with_default_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            vec![RouteRule::new_static("R2Northstar/mods").with_subdir(false)],
            None,
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }

    pub fn add_install_rule(&mut self, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.add(rule.into());
    }

    pub fn insert_install_rule(&mut self, index: usize, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.insert(index, rule.into());
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
                    .use_links(true)
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
    use loadsmith_core::PackageRef;

    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            Northstar::with_default_rules(),
            PackageRef::new("northstar-Northstar".to_string(), (1, 31, 10)),
            true,
        ), [
            "README.md" => None,
            "Northstar/r2ds.bat" => "./r2ds.bat",
            "Northstar/R2Northstar/file.txt" => "./R2Northstar/file.txt",
            "Northstar/R2Northstar/plugins/DiscordRPC.dll" => "./R2Northstar/plugins/DiscordRPC.dll",
        ])
    }

    // #[test]
    // fn map_package_files() {
    //     assert_maps!(MapFileTester::new(
    //         Northstar::with_default_rules(),
    //         PackageRef::new("Author-Name".to_string(), (1, 0, 0)),
    //         false,
    //     ), [
    //         "README.md" => "BepInEx/plugins/Author-Name/README.md",
    //         "nested/file.txt" => "BepInEx/plugins/Author-Name/file.txt",
    //         "plugins/nested/file.txt" => "BepInEx/plugins/Author-Name/nested/file.txt",
    //         "config/settings.json" => "BepInEx/config/settings.json",
    //         "patchers/patcher.dll" => "BepInEx/patchers/Author-Name/patcher.dll",
    //         "core/core.dll" => "BepInEx/core/Author-Name/core.dll",
    //         "patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
    //         "nested/patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
    //         "monomod/patch.dll" => "BepInEx/monomod/Author-Name/patch.dll",
    //         "monomod/nested/patch.dll" => "BepInEx/monomod/Author-Name/nested/patch.dll"
    //     ]);
    // }

    #[test]
    fn package_dir_works() {
        let loader = Northstar::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), (1, 0, 0));

        assert!(loader.package_dir(&package).is_none());
    }
}
