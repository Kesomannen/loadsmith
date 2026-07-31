use std::{path::PathBuf, sync::LazyLock};

use camino::Utf8PathBuf;
use globset::{GlobBuilder, GlobSet};
use loadsmith_install::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{Error, LaunchArgs, LaunchContext, Loader, Result, doorstop, glob, glob_rule};

/// Loader implementation for [BepInEx](https://docs.bepinex.dev/), a
/// general-purpose Unity mod loader.
///
/// BepInEx uses Doorstop to inject into the game process and supports several
/// sub-directories for plugins, patchers, monomod assemblies, and config.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::{BepInEx, Loader};
///
/// let loader = BepInEx::with_default_rules();
/// assert_eq!(loader.id(), "BepInEx");
/// ```
#[derive(Debug, Clone)]
pub struct BepInEx {
    package_install_ruleset: OwnedInstallRuleset,
}

impl BepInEx {
    /// Creates a `BepInEx` loader with a custom install ruleset.
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    /// Creates a `BepInEx` loader with the default install rules.
    ///
    /// Default routes:
    ///
    /// * `BepInEx/config` (mutable, flat)
    /// * `BepInEx/patchers`
    /// * `BepInEx/core`
    /// * `BepInEx/monomod` (files with `.mm.dll` extension)
    /// * `BepInEx/plugins` (files with `.dll` extension) — **default rule**
    pub fn with_default_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            vec![
                RouteRule::new_static("BepInEx/config")
                    .with_subdir(false)
                    .with_mutable(true),
                RouteRule::new_static("BepInEx/patchers"),
                RouteRule::new_static("BepInEx/core"),
                RouteRule::new_static("BepInEx/monomod").with_file_extension("mm.dll"),
                RouteRule::new_static("BepInEx/plugins").with_file_extension("dll"),
            ],
            Some(4),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }

    /// Adds an install rule to the end of the ruleset.
    pub fn add_install_rule(&mut self, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.add_rule(rule.into());
    }

    /// Inserts an install rule at the given index.
    pub fn insert_install_rule(&mut self, index: usize, rule: impl Into<InstallRule>) {
        self.package_install_ruleset.insert_rule(index, rule.into());
    }
}

impl Default for BepInEx {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl Loader for BepInEx {
    fn id(&self) -> &'static str {
        "BepInEx"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("*/*.{ini,cfg}" => ".")
                    .strip_top_level(true)
                    .into(),
                glob_rule!("*/*" => ".")
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
        static INCLUDE_SET: LazyLock<GlobSet> = LazyLock::new(|| {
            GlobSet::builder()
                .add(
                    // copy all loose files
                    GlobBuilder::new("*")
                        .literal_separator(true)
                        .build()
                        .expect("constant glob should be valid"),
                )
                .add(glob!("doorstop_libs/*"))
                .add(glob!("dotnet/*"))
                .add(glob!("corlibs/*"))
                .build()
                .expect("constant globs should be valid")
        });

        ctx.copy_glob_to_game(&INCLUDE_SET)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        let (enable_prefix, target_prefix) = doorstop::args(None, ctx)?;
        let preloader_path = bepinex_preloader_path(None, ctx)?;

        let args = LaunchArgs::new()
            .arg(enable_prefix)
            .arg("true")
            .arg(target_prefix)
            .arg(&*ctx.format_proton_path(&preloader_path));

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        vec!["BepInEx/config".into()]
    }

    fn log_file(&self) -> Option<PathBuf> {
        Some("BepInEx/BepInEx.log".into())
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        Some("winhttp.dll".into())
    }
}

pub(crate) fn bepinex_preloader_path(
    prefix: Option<&str>,
    ctx: &LaunchContext,
) -> Result<Utf8PathBuf> {
    let mut core_directory = ctx.profile_path().to_path_buf();

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
        .ok_or(Error::BepInExPreloaderNotFound { core_directory })?;

    let path = Utf8PathBuf::from_path_buf(entry.path())
        .expect("BepInEx core directory should be valid UTF-8");

    Ok(path)
}

#[cfg(test)]
mod tests {
    use loadsmith_core::{PackageRef, Version};

    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            BepInEx::with_default_rules(),
            PackageRef::new("BepInEx-BepInExPack".to_string(), Version::new(5, 4, 2100)),
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
            BepInEx::with_default_rules(),
            PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0)),
            false,
        ), [
            "README.md" => "BepInEx/plugins/Author-Name/README.md",
            "nested/file.txt" => "BepInEx/plugins/Author-Name/file.txt",
            "plugins/nested/file.txt" => "BepInEx/plugins/Author-Name/nested/file.txt",
            "BepInEx/plugins/file.txt" => "BepInEx/plugins/Author-Name/file.txt",
            "config/settings.json" => "BepInEx/config/settings.json",
            "BepInEx/config/myconfig.cfg" => "BepInEx/config/myconfig.cfg",
            "patchers/patcher.dll" => "BepInEx/patchers/Author-Name/patcher.dll",
            "core/core.dll" => "BepInEx/core/Author-Name/core.dll",
            "patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
            "nested/patch.mm.dll" => "BepInEx/monomod/Author-Name/patch.mm.dll",
            "monomod/patch.dll" => "BepInEx/monomod/Author-Name/patch.dll",
            "monomod/nested/patch.dll" => "BepInEx/monomod/Author-Name/nested/patch.dll"
        ]);
    }

    #[test]
    fn config_files_should_not_link() {
        let loader = BepInEx::with_default_rules();
        let rules = loader.package_install_rules();

        assert!(
            !rules
                .find_rule_for_path("config/settings.json")
                .unwrap()
                .use_links()
        );
    }

    #[test]
    fn other_files_should_link() {
        let loader = BepInEx::with_default_rules();
        let rules = loader.package_install_rules();

        assert!(
            rules
                .find_rule_for_path("plugins/file.txt")
                .unwrap()
                .use_links()
        );

        assert!(
            rules
                .find_rule_for_path("patchers/patch.dll")
                .unwrap()
                .use_links()
        );

        assert!(
            rules
                .find_rule_for_path("core/core.dll")
                .unwrap()
                .use_links()
        );

        assert!(
            rules
                .find_rule_for_path("patch.mm.dll")
                .unwrap()
                .use_links()
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = BepInEx::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0));

        let package_dir = loader.package_dir(&package).unwrap();

        assert_eq!(package_dir, PathBuf::from("BepInEx/plugins/Author-Name"));
    }
}
