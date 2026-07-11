use std::{path::PathBuf, sync::LazyLock, vec};

use loadsmith_install::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

#[derive(Debug, Clone)]
pub struct Shimloader {
    package_install_ruleset: OwnedInstallRuleset,
}

impl Shimloader {
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    pub fn with_default_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            [
                RouteRule::new_static("shimloader/mod"),
                RouteRule::new_static("shimloader/pak").with_file_extension("pak"),
                RouteRule::new_static("shimloader/cfg")
                    .with_subdir(false)
                    .with_mutable(true),
                RouteRule::new_static("shimloader/overlay"),
            ],
            Some(0),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }
}

impl Default for Shimloader {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl Loader for Shimloader {
    fn id(&self) -> &'static str {
        "Shimloader"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("UE4SS/Mods/*" => "shimloader/mod")
                    .use_links(true)
                    .strip_levels(2)
                    .into(),
                glob_rule!("UE4SS/UE4SS.dll" => ".")
                    .use_links(true)
                    .strip_levels(1)
                    .into(),
                glob_rule!("UE4SS/UE4SS-settings.ini" => ".")
                    .use_links(true)
                    .strip_levels(1)
                    .into(),
                glob_rule!("dwmapi.dll" => ".").use_links(true).into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.package_install_ruleset.as_ref()
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let path = ctx.profile_path().join("shimloader");

        let mod_path = path.join("mod");
        let pak_path = path.join("pak");
        let cfg_path = path.join("cfg");

        let mod_path = ctx.format_proton_path(&mod_path);
        let pak_path = ctx.format_proton_path(&pak_path);
        let cfg_path = ctx.format_proton_path(&cfg_path);

        let args = LaunchArgs::new()
            .arg("--mod-dir")
            .arg(&*mod_path)
            .arg("--pak-dir")
            .arg(&*pak_path)
            .arg("--cfg-dir")
            .arg(&*cfg_path);

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        vec!["shimloader/cfg".into()]
    }

    fn log_file(&self) -> Option<PathBuf> {
        None
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        Some("dwmapi.dll".into())
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
            Shimloader::with_default_rules(),
            PackageRef::new("Thunderstore-unreal_shimloader".to_string(), (1, 1, 7)),
            true,
        ), [
            "README.md" => None,
            "dwmapi.dll" => "./dwmapi.dll",
            "UE4SS/dwmapi.dll" => None,
            "UE4SS/UE4SS.dll" => "./UE4SS.dll",
            "UE4SS/UE4SS-settings.ini" => "./UE4SS-settings.ini",
            "UE4SS/Mods/mods.json" => "shimloader/mod/mods.json",
            "UE4SS/Mods/mods.txt" => "shimloader/mod/mods.txt",
            "UE4SS/Mods/shared/scripts/script.lua" => "shimloader/mod/shared/scripts/script.lua",
        ]);
    }

    #[test]
    fn map_package_files() {
        assert_maps!(
            MapFileTester::new(
                Shimloader::with_default_rules(),
                PackageRef::new("Author-Name".to_string(), (1, 0, 0)),
                false,
            ),
            [
                "README.md" => "shimloader/mod/Author-Name/README.md",
                "pak/file" => "shimloader/pak/Author-Name/file",
                "cfg/settings.json" => "shimloader/cfg/settings.json",
                "nested/file.txt" => "shimloader/mod/Author-Name/file.txt",
                "mypak.pak" => "shimloader/pak/Author-Name/mypak.pak",
            ]
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = Shimloader::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), (1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(
            package_dir,
            Some(PathBuf::from("shimloader/mod/Author-Name"))
        );
    }
}
