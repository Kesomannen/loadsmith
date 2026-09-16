use std::{path::PathBuf, sync::LazyLock};

use camino::Utf8Path;
use globset::GlobSet;
use loadsmith_install::rule::{InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};

use crate::{LaunchArgs, LaunchContext, Loader, Result, glob_rule};

/// Loader implementation for [MelonLoader](https://melonwiki.xyz/), a universal mod loader for
/// Unity games, supporting both Mono and IL2CPP.
///
/// MelonLoader supports two package installation schemes:
///
/// * **Legacy** — routes files to `UserLibs`, `Plugins`, `MelonLoader/Managed`,
///   `MelonLoader/Libs`, `MelonLoader`, `Mods`, and `UserData/ModManager`.
/// * **Recursive** — routes all files into `Mods` and keeps `UserData` for
///   mutable config.
///
/// At launch, the game is pointed at the profile directory with the `--melonloader.basedir`
/// argument, and `--melonloader.agfregenerate` is added when the game's assemblies have not
/// been generated yet.
///
/// The package installation rules for MelonLoader are configurable, with a set of default rules
/// for each scheme. You'll find a list of the rules in the documentation for
/// [`with_default_legacy_rules`](MelonLoader::with_default_legacy_rules) and
/// [`with_default_recursive_rules`](MelonLoader::with_default_recursive_rules).
#[derive(Debug, Clone)]
pub struct MelonLoader {
    package_install_ruleset: OwnedInstallRuleset,
}

impl MelonLoader {
    /// Creates a `MelonLoader` loader with a custom install ruleset.
    pub fn with_rules(package_install_ruleset: OwnedInstallRuleset) -> Self {
        Self {
            package_install_ruleset,
        }
    }

    /// Creates a `MelonLoader` with the legacy install rules (original
    /// per-directory routing).
    ///
    /// |                     | Subdir | Flattened | Mutable | Extension     | Default |
    /// |---------------------|--------|-----------|---------|---------------|---------|
    /// | UserLibs            | `-`    | `X`       | `-`     | `lib.dll`     | `-`     |
    /// | Plugins             | `-`    | `X`       | `-`     | `plugin.dll`  | `-`     |
    /// | MelonLoader/Managed | `-`    | `X`       | `-`     | `managed.dll` | `-`     |
    /// | MelonLoader/Libs    | `-`    | `X`       | `-`     | `-`           | `-`     |
    /// | MelonLoader         | `-`    | `X`       | `-`     | `-`           | `-`     |
    /// | Mods                | `-`    | `X`       | `-`     | `dll`         | `-`     |
    /// | UserData/ModManager | `X`    | `-`       | `X`     | `-`           | `X`     |
    ///
    /// Read more about how rules work in the documentation of the [`InstallRule`] struct.
    pub fn with_default_legacy_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            [
                RouteRule::new_static("UserLibs")
                    .with_file_extension("lib.dll")
                    .with_subdir(false),
                RouteRule::new_static("Plugins")
                    .with_file_extension("plugin.dll")
                    .with_subdir(false),
                RouteRule::new_static("MelonLoader/Managed")
                    .with_file_extension("managed.dll")
                    .with_subdir(false),
                RouteRule::new_static("MelonLoader/Libs").with_subdir(false),
                RouteRule::new_static("MelonLoader").with_subdir(false),
                RouteRule::new_static("Mods")
                    .with_file_extension("dll")
                    .with_subdir(false),
                RouteRule::new_static("UserData/ModManager")
                    .with_flatten(false)
                    .with_mutable(true),
            ],
            Some(6),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }

    /// Creates a `MelonLoader` with the recursive install rules (all files go
    /// into `Mods`).
    ///
    /// |          | Subdir | Flattened | Mutable | Extension | Default |
    /// |----------|--------|-----------|---------|-----------|---------|
    /// | Mods     | `X`    | `-`       | `-`     | `-`       | `X`     |
    /// | UserData | `X`    | `X`       | `X`     | `-`       | `-`     |
    ///
    /// The `Mods` rule has an empty route name, meaning it matches every file and acts as the
    /// fallback for files that no other rule matches.
    ///
    /// Read more about how rules work in the documentation of the [`InstallRule`] struct.
    pub fn with_default_recursive_rules() -> Self {
        OwnedInstallRuleset::from_rule_iter(
            [
                RouteRule::new_with_target("", Utf8Path::new("Mods")).with_flatten(false),
                RouteRule::new_static("UserData").with_mutable(true),
            ],
            Some(0),
        )
        .map(Self::with_rules)
        .expect("rules are not empty so there should always be a valid default rule index")
    }

    /// Sets a glob set of files to exclude from installation.
    pub fn set_exclude(&mut self, exclude: GlobSet) {
        self.package_install_ruleset.set_exclude(exclude);
    }

    /// Adds a rule to the end of the package install ruleset.
    pub fn add_install_rule(&mut self, rule: InstallRule) {
        self.package_install_ruleset.add_rule(rule);
    }
}

impl Default for MelonLoader {
    fn default() -> Self {
        Self::with_default_legacy_rules()
    }
}

impl Loader for MelonLoader {
    fn id(&self) -> &'static str {
        "MelonLoader"
    }

    fn loader_install_rules(&self) -> InstallRuleset<'_> {
        static RULES: LazyLock<Vec<InstallRule>> = LazyLock::new(|| {
            vec![
                glob_rule!("MelonLoader/{Dependencies,Documentation,net*}/*" => ".")
                    .with_links(true)
                    .into(),
                glob_rule!("{version,dobby}.dll" => ".").into(),
            ]
        });

        InstallRuleset::new(&RULES)
    }

    fn package_install_rules(&self) -> InstallRuleset<'_> {
        self.package_install_ruleset.as_ref()
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
    use loadsmith_core::{PackageRef, Version};

    use crate::{assert_map, assert_maps, test_util::MapFileTester};

    use super::*;

    #[test]
    fn map_loader_files() {
        assert_maps!(MapFileTester::new(
            MelonLoader::with_default_legacy_rules(),
            PackageRef::new("LavaGang-MelonLoader".to_string(), Version::new(1, 0, 0)),
            true,
        ), [
            "README.md" => None,
            "version.dll" => "./version.dll",
            "MelonLoader/Dependencies/Name/file.dll" => "./MelonLoader/Dependencies/Name/file.dll",
            "MelonLoader/Documentation/file.txt" => "./MelonLoader/Documentation/file.txt",
            "MelonLoader/net6/file.dll" => "./MelonLoader/net6/file.dll",
            "MelonLoader/net35/file.dll" => "./MelonLoader/net35/file.dll",
            "MelonLoader/net472/file.dll" => "./MelonLoader/net472/file.dll",
        ])
    }

    #[test]
    fn map_package_files() {
        assert_maps!(MapFileTester::new(
            MelonLoader::with_default_legacy_rules(),
            PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0)),
            false,
        ), [
            "UserLibs/file" => "UserLibs/file",
            "UserLibs/nested/file" => "UserLibs/nested/file",
            "userlibrary.lib.dll" => "UserLibs/userlibrary.lib.dll",
            "Managed/file" => "MelonLoader/Managed/file",
            "Managed/nested/file" => "MelonLoader/Managed/nested/file",
            "managed.managed.dll" => "MelonLoader/Managed/managed.managed.dll",
            "Mods/file" => "Mods/file",
            "Mods/nested/file" => "Mods/nested/file",
            "mod.dll" => "Mods/mod.dll",
            "manifest.json" => "UserData/ModManager/Author-Name/manifest.json",
            "ModManager/file" => "UserData/ModManager/Author-Name/file",
            "ModManager/nested/file" => "UserData/ModManager/Author-Name/nested/file",
            "MelonLoader/file" => "MelonLoader/file",
            "MelonLoader/nested/file" => "MelonLoader/nested/file",
            "MelonLoader/Libs/file" => "MelonLoader/Libs/file",
            "MelonLoader/Libs/nested/file" => "MelonLoader/Libs/nested/file",
            "MelonLoader/Libs/lib.dll" => "MelonLoader/Libs/lib.dll",
        ]);
    }

    #[test]
    fn package_dir_works() {
        let loader = MelonLoader::with_default_legacy_rules();
        let package = PackageRef::new("Author-Name".to_string(), Version::new(1, 0, 0));

        let package_dir = loader.package_dir(&package);

        assert_eq!(
            package_dir,
            Some(PathBuf::from("UserData/ModManager/Author-Name"))
        );
    }
}
