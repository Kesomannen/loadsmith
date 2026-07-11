use std::{path::PathBuf, sync::LazyLock};

use camino::Utf8Path;
use globset::GlobSet;
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

    fn prepare_launch(&self, ctx: &LaunchContext) -> Result<()> {
        static GLOB_SET: LazyLock<GlobSet> = LazyLock::new(|| {
            GlobSet::builder()
                .add(super::top_level_dll_glob())
                .build()
                .expect("constant globs should be valid")
        });

        ctx.copy_glob_to_game(&GLOB_SET)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let mut args = LaunchArgs::new()
            .arg("--melonloader.basedir")
            .arg(ctx.profile_path());

        let mono_assembly_exists = ctx
            .profile_path()
            .join("MelonLoader/Managed/Assembly-CSharp.dll")
            .exists();
        let il2cpp_assembly_exists = ctx
            .profile_path()
            .join("MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll")
            .exists();

        if !mono_assembly_exists && !il2cpp_assembly_exists {
            args = args.arg("--melonloader.agfregenerate");
        }

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn log_file(&self) -> Option<PathBuf> {
        Some("MelonLoader/Latest.log".into())
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
                "README.md" => "shimloader/mod/README.md",
                "pak/file" => "shimloader/pak/file",
                "cfg/settings.json" => "shimloader/cfg/settings.json",
                "nested/file.txt" => "shimloader/mod/file.txt",
                "mypak.pak" => "shimloader/pak/mypak.pak",
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
