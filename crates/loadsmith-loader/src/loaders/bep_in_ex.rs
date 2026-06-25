use std::{borrow::Cow, sync::LazyLock};

use camino::{Utf8Path, Utf8PathBuf};
use loadsmith_core::{LaunchArgs, LoaderId};
use loadsmith_install::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{Error, LaunchContext, Loader, Result, doorstop, glob_rule, glob_rules};

#[derive(Debug, Clone)]
pub struct BepInExLoader {
    package_install_ruleset: OwnedInstallRuleset,
}

impl BepInExLoader {
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    pub fn with_default_rules() -> Self {
        OwnedInstallRuleset::with_rules(
            vec![
                InstallRule::Route(RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"))),
                InstallRule::Route(
                    RouteRule::new("config", Utf8Path::new("BepInEx/config")).with_subdir(false),
                ),
                InstallRule::Route(RouteRule::new(
                    "patchers",
                    Utf8Path::new("BepInEx/patchers"),
                )),
                InstallRule::Route(RouteRule::new("core", Utf8Path::new("BepInEx/core"))),
                InstallRule::Route(
                    RouteRule::new("monomod", Utf8Path::new("BepInEx/monomod"))
                        .with_file_extensions(vec![Cow::Borrowed("mm.dll")]),
                ),
            ],
            Some(0),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }

    pub fn add_install_rule(&mut self, rule: InstallRule) {
        self.package_install_ruleset.add(rule);
    }
}

impl Default for BepInExLoader {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl Loader for BepInExLoader {
    fn id(&self) -> LoaderId {
        LoaderId("BepInEx")
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> =
            LazyLock::new(|| glob_rules![("*/**" => ".", true)]);

        InstallRuleset::new(&*RULES, None)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.package_install_ruleset.as_ref()
    }

    fn get_launch_args(&self, ctx: LaunchContext) -> Result<LaunchArgs> {
        let (enable_prefix, target_prefix) = doorstop::args(None, &ctx)?;
        let preloader_path = bepinex_preloader_path(None, &ctx)?;

        let args = LaunchArgs::new()
            .arg(enable_prefix)
            .arg("true")
            .arg(target_prefix)
            .arg(&*ctx.format_proton_path(&preloader_path));

        Ok(args)
    }
}

pub(crate) fn bepinex_preloader_path(
    prefix: Option<&str>,
    ctx: &LaunchContext,
) -> Result<Utf8PathBuf> {
    let mut core_directory = ctx.profile.to_path_buf();

    if let Some(prefix) = prefix {
        core_directory.push(prefix);
    }

    core_directory.push("BepInEx");
    core_directory.push("core");

    const PRELOADER_NAMES: &[&str] = &[
        "BepInEx.Unity.Mono.Preloader.dll",
        "BepInEx.Unity.IL2CPP.dll",
        "BepInEx.Preloader.dll",
        "BepInEx.IL2CPP.dll",
    ];

    let entry = core_directory
        .read_dir()
        .map_err(|err| Error::BepInExCoreDirectoryMissing { source: err })?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            let file_name = entry.file_name();
            PRELOADER_NAMES.iter().any(|name| file_name == **name)
        })
        .ok_or_else(|| Error::BepInExPreloaderNotFound { core_directory })?;

    let path = Utf8PathBuf::from_path_buf(entry.path())
        .expect("BepInEx core directory should be valid UTF-8");

    Ok(path)
}

#[cfg(test)]
mod tests {
    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            BepInExLoader::with_default_rules(),
            "BepInEx-BepInExPack",
            "5.4.2100",
            true,
        ), [
            "README.md" => None,
            "BepInExPack/doorstop_config.ini" => "./doorstop_config.ini",
            "BepInExPack/BepInEx/core/BepInEx.Preloader.dll" => "./BepInEx/core/BepInEx.Preloader.dll"
        ])
    }

    #[test]
    fn map_package_files() {
        assert_maps!(MapFileTester::new(
            BepInExLoader::with_default_rules(),
            "Author-Name",
            "1.0.0",
            false,
        ), [
            "README.md" => "BepInEx/plugins/Author-Name/README.md",
            "nested/file.txt" => "BepInEx/plugins/Author-Name/file.txt",
            "plugins/nested/file.txt" => "BepInEx/plugins/Author-Name/nested/file.txt",
            "config/settings.json" => "BepInEx/config/settings.json",
            "patchers/patcher.dll" => "BepInEx/patchers/Author-Name/patcher.dll",
            "core/core.dll" => "BepInEx/core/Author-Name/core.dll",
            "patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
            "nested/patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
            "monomod/patch.dll" => "BepInEx/monomod/Author-Name/patch.dll",
            "monomod/nested/patch.dll" => "BepInEx/monomod/Author-Name/nested/patch.dll"
        ]);
    }
}
