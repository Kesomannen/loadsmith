use std::{path::PathBuf, sync::LazyLock};

use camino::Utf8Path;
use globset::GlobBuilder;
use loadsmith_install::{GlobRule, InstallRule, InstallRuleset};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

#[derive(Debug, Clone)]
pub struct GDWeave;

impl GDWeave {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GDWeave {
    fn default() -> Self {
        Self::new()
    }
}

impl Loader for GDWeave {
    fn id(&self) -> &'static str {
        "GDWeave"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("GDWeave/*" => ".").use_links(true).into(),
                glob_rule!("*.dll" => ".").use_links(true).into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                GlobRule::new(
                    GlobBuilder::new("GDWeave/mods/*/**")
                        .case_insensitive(true)
                        .literal_separator(true)
                        .build()
                        .expect("constant glob should be valid"),
                    Utf8Path::new("GDWeave/mods"),
                )
                .strip_levels(3)
                .with_subdir(true)
                .use_links(true)
                .into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let gd_weave = ctx.profile_path().join("GDWeave");
        let path = ctx.format_proton_path(&gd_weave);

        let args = LaunchArgs::new().arg(format!("--gdweave-folder-override={path}"));

        Ok(args)
    }

    fn package_dir(&self, package: &loadsmith_core::PackageRef) -> Option<PathBuf> {
        Some(PathBuf::from("GDWeave/mods").join(package.id().as_str()))
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        vec!["GDWeave/configs".into()]
    }

    fn log_file(&self) -> Option<PathBuf> {
        Some("GDWeave/GDWeave.log".into())
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        Some("winmm.dll".into())
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
            GDWeave::new(),
            PackageRef::new("NotNet-GDWeave".to_string(), (2, 0, 14)),
            true,
        ), [
            "README.md" => None,
            "winmm.dll" => "./winmm.dll",
            "GDWeave/core/GDWeave.dll" => "./GDWeave/core/GDWeave.dll",
            "GDWeave/core/GDWeave.pub" => "./GDWeave/core/GDWeave.pub",
        ]);
    }

    #[test]
    fn map_package_files() {
        assert_maps!(
            MapFileTester::new(
                GDWeave::new(),
                PackageRef::new("Author-Name".to_string(), (1, 0, 0)),
                false,
            ),
            [
                "manifest.json" => None,
                "other/file.txt" => None,
                "mod/other/file.txt" => None,
                "mod/other/nested/file.txt" => None,
                "GDWeave/mods/Author.Name/manifest.json" => "GDWeave/mods/Author-Name/manifest.json",
                "GDWeave/mods/Author.Name/nested/file.txt" => "GDWeave/mods/Author-Name/nested/file.txt",
                "GDWeave/mods/Author.Name/1/2/file.txt" => "GDWeave/mods/Author-Name/1/2/file.txt",
                "gdweave/mods/author.name/file.txt" => "GDWeave/mods/Author-Name/file.txt",
                "GDWEAVE/MODS/AUTHOR.NAME/FILE.txt" => "GDWeave/mods/Author-Name/FILE.txt",
            ]
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = GDWeave::new();
        let package = PackageRef::new("Author-Name".to_string(), (1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(package_dir, Some(PathBuf::from("GDWeave/mods/Author-Name")));
    }
}
