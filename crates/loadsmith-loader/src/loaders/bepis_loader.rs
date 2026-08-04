use std::path::PathBuf;

use camino::Utf8Path;
use loadsmith_install::rule::{InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{BepInEx, LaunchArgs, LaunchContext, Loader, Result};

/// Loader implementation for BepisLoader, a mod loader for Honey Select /
/// Koikatsu that wraps BepInEx with a renderer-specific plugin directory.
///
/// BepisLoader adds a `renderer` route pointing at `Renderer/BepInEx/plugins`
/// on top of the standard BepInEx rules. It also forces Doorstop version 4.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::{BepisLoader, Loader};
///
/// let loader = BepisLoader::with_default_rules();
/// assert_eq!(loader.id(), "BepisLoader");
/// ```
#[derive(Debug, Clone)]
pub struct BepisLoader {
    inner: BepInEx,
}

impl BepisLoader {
    fn new(inner: BepInEx) -> Self {
        Self { inner }
    }

    /// Creates a `BepisLoader` with a custom install ruleset.
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self::new(BepInEx::with_rules(package_install_ruleset))
    }

    /// Creates a `BepisLoader` with the default rules (BepInEx defaults plus a
    /// `renderer` route).
    pub fn with_default_rules() -> Self {
        let mut inner = BepInEx::with_default_rules();
        inner.insert_install_rule(
            0,
            RouteRule::new_with_target("renderer", Utf8Path::new("Renderer/BepInEx/plugins")),
        );
        Self::new(inner)
    }
}

impl Loader for BepisLoader {
    fn id(&self) -> &'static str {
        "BepisLoader"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        self.inner.loader_install_rules()
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.inner.package_install_rules()
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs> {
        // Don't use format_path as BepisLoader escapes Proton and therefore
        // does not recognize the Z: prefix, see https://github.com/Kesomannen/gale/issues/627
        let bepinex_target = ctx.profile_path().join("BepInEx");

        let mut args = LaunchArgs::new()
            .arg("--hookfxr-enable")
            .arg("--bepinex-target")
            .arg(bepinex_target);

        let preloader = super::bep_in_ex::bepinex_preloader_path(Some("Renderer"), ctx)?;
        let preloader = ctx.format_proton_path(&preloader);

        let (enable_prefix, target_prefix) = crate::doorstop::args(Some(4), ctx)?;
        args = args
            .arg(enable_prefix)
            .arg("true")
            .arg(target_prefix)
            .arg(&*preloader);

        Ok(args)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        vec!["BepInEx/config".into(), "Renderer/BepInEx/config".into()]
    }

    fn log_file(&self) -> Option<PathBuf> {
        self.inner.log_file()
    }

    fn proxy_dll(&self) -> Option<PathBuf> {
        self.inner.proxy_dll()
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
            BepisLoader::with_default_rules(),
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
            BepisLoader::with_default_rules(),
            PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0)),
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
            "monomod/nested/patch.dll" => "BepInEx/monomod/Author-Name/nested/patch.dll",
            "renderer/patch.dll" => "Renderer/BepInEx/plugins/Author-Name/patch.dll",
        ]);
    }

    #[test]
    fn config_files_should_not_link() {
        let loader = BepisLoader::with_default_rules();
        let rules = loader.package_install_rules();

        assert!(
            !rules
                .find_rule_for_path("config/settings.json")
                .unwrap()
                .use_links()
        );
    }

    #[test]
    fn package_dir_works() {
        let loader = BepisLoader::with_default_rules();
        let package = PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0));

        let package_dir = loader.package_dir(&package).unwrap();

        assert_eq!(package_dir, PathBuf::from("BepInEx/plugins/Author-Name"));
    }
}
