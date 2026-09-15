use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

use crate::ConflictStrategy;

/// A highly configurable route-based install rule.
///
/// A rule is defined by two main parameters: the route `name` and `target`. These are usually derived
/// from the same path, but can be set independently.
///
/// The rule will search a given file path case-insensitively for a component matching the route `name`. If found, the part of
/// the path before the route name (the "prelude") is optionally stripped and the _remainder_ is appended to the target path.
/// Additionally, a subdirectory named after the package's ID may be inserted between the target and the _remainder_.
///
/// Example decomposition of path with a route name of `plugins` and target `BepInEx/plugins`:
///
/// ```not_rust
/// MyFolder/MyFolder2/Plugins/AnotherFolder/ExtraFolder/MyPlugin.dll
/// |-----prelude-----|--name-|--------remainder--------|--filename-|
/// ```
///
/// With the default configuration, this path would be mapped to the following install destination:
///
/// ```not_rust
/// BepInEx/plugins/Author-Name/AnotherFolder/ExtraFolder/MyPlugin.dll
/// |----target----|--subdir--|--------remainder---------|--filename-|
/// ```
///
/// However, this behavior can be heavily customized with the various options available the struct.
///
/// ## Background
///
/// This struct is designed to cover the installation logic used for mods in most mod loaders/games on the Thunderstore platform.
/// One example is the BepInEx extraction rules, which is used for a majority of games on the platform, including Lethal Company,
/// Valheim and R.E.P.O. to name a few. These rules are described in this [r2modman wiki article].
///
/// The main motivation behind these rules is to prevent file conflicts between mods, while still
/// allowing for a good amount of flexibility in how mods are structured and installed.
///
/// Take for instance BepInEx. A typical manual BepInEx installation may look like the following:
///
/// ```not_rust
/// <profile/game folder>
/// |-- BepInEx
///     |-- plugins
///     |   |-- MonoMod-Plugin.dll
///     |   |-- SomeOtherMod.dll
///     |-- patchers
///     |   |-- MonoMod-Patcher.dll
///     |-- core
///     |   |-- <internal BepInEx files>
///     |   |-- ModThatRequiresCore.dll
///     |-- config
///         |-- BepInEx.cfg
///         |-- MonoMod.cfg
/// ```
///
/// Say we want a mod manager to install mods in this configuration using standard zip extraction rules.
/// For example, the mod `ModThatRequiresCore` may be packaged in a zip file like this:
///
/// ```not_rust
/// SomeAuthor-ModThatRequiresCore.zip
/// |-- BepInEx
///     |-- core
///         |-- ModThatRequiresCore.dll
/// ```
///
/// But what happens if another mod also has a file named `ModThatRequiresCore.dll` at the same location?
///
/// If the mod manager simply extracts the zip file into the game folder, it will overwrite the existing file and break the already-installed mod.
/// With route-based rules, the mod manager can instead install the file into a subdirectory named after the mod's package ID, like this:
///
/// ```not_rust
/// <profile/game folder>
/// |-- BepInEx
///     |-- plugins
///     |   |-- SomeAuthor-MonoMod
///     |   |   |-- MonoMod-Plugin.dll
///     |   |-- SomeOtherAuthor-SomeOtherMod
///     |   |   |-- SomeOtherMod.dll
///     |-- patchers
///     |   |-- SomeAuthor-MonoMod
///     |   |   |-- MonoMod-Patcher.dll
///     |-- core
///     |   |-- <internal BepInEx files>
///     |   |-- SomeAuthor-ModThatRequiresCore
///     |   |   |-- ModThatRequiresCore.dll
///     |-- config
///         |-- BepInEx.cfg
///         |-- MonoMod.cfg
/// ```
///
/// With route rules, we would express this with the following four rules:
///
/// ```
/// # use loadsmith_install::rule::RouteRule;
/// let plugins_rule = RouteRule::new_static("BepInEx/plugins");
/// let patchers_rule = RouteRule::new_static("BepInEx/patchers");
/// let core_rule = RouteRule::new_static("BepInEx/core");
/// // By default, files are placed with a subdirectory named after the package ID.
/// // We don't want this for `config` files, disable it specifically for that rule.
/// let config_rule = RouteRule::new_static("BepInEx/config").with_subdir(false);
/// ```
///
/// This is the basic principle of route-based rules. However, there are many more options available to capture
/// the nuances of Thunderstore mod installation. See the following methods for more information:
/// - [`with_file_extension`][RouteRule::with_file_extension]
/// - [`with_file_extensions`][RouteRule::with_file_extensions]
/// - [`with_flatten`][RouteRule::with_flatten]
/// - [`with_subdir`][RouteRule::with_subdir]
/// - [`with_mutable`][RouteRule::with_mutable]
///
/// [r2modman wiki article]: https://github.com/ebkr/r2modmanPlus/wiki/Structuring-your-Thunderstore-package
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRule {
    name: String,
    pub(super) target: Utf8PathBuf,
    file_extensions: Vec<String>,
    subdir: bool,
    flatten: bool,
    mutable: bool,
}

impl RouteRule {
    /// Creates a route rule from a static path string. The rule name is derived
    /// from the last path component.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_install::rule::RouteRule;
    /// let rule = RouteRule::new_static("BepInEx/plugins");
    /// assert_eq!(rule.name(), "plugins");
    /// assert_eq!(rule.target(), "BepInEx/plugins");
    ///
    /// let rule = RouteRule::new_static("MelonLoader");
    /// assert_eq!(rule.name(), "MelonLoader");
    /// assert_eq!(rule.target(), "MelonLoader");
    /// ```
    pub fn new_static(path: &'static str) -> Self {
        Self::new(Cow::Borrowed(Utf8Path::new(path)))
    }

    /// Creates a route rule from any 'static or allocated path. The rule name is derived from the
    /// last path component.
    ///
    /// If you simply want to create a rule from a static string, use [`new_static`](RouteRule::new_static) instead.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_install::rule::RouteRule;
    /// let mut path = camino::Utf8PathBuf::new();
    /// path.push("BepInEx");
    /// path.push("plugins");
    ///
    /// let rule = RouteRule::new(path);
    /// assert_eq!(rule.name(), "plugins");
    /// assert_eq!(rule.target(), "BepInEx/plugins");
    /// ```
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        let path = path.into();

        let name = path
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_default();

        Self::new_with_target(name, path)
    }

    /// Creates a route rule with an explicit name and target path.
    ///
    /// The `name` is the directory component that triggers a match; `target` is
    /// the directory where matched files are installed.
    pub fn new_with_target(name: impl Into<String>, target: impl Into<Utf8PathBuf>) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            file_extensions: Vec::new(),
            flatten: true,
            subdir: true,
            mutable: false,
        }
    }

    /// Returns the route name that triggers a match.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the target path where matched files are installed.
    pub fn target(&self) -> &Utf8Path {
        &self.target
    }

    /// Specify a file extension that will match the rule, in addition to the route name.
    ///
    /// This _does not_ change the behaviour of `map_file`, only which paths are said to match the rule.
    /// This is notable for [`InstallRuleset`](super::InstallRuleset), as it uses this method to determine which rule to apply for a
    /// given file.
    ///
    /// Note that this also _does not_ use `std`s definition of a file extension. [`Path::extension`](std::path::Path::extension)
    /// and similar methods only return the part after the last dot. `RouteRule`, on the other hand, considers the entire part after
    /// the first dot in the file name to be the extension.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_install::rule::RouteRule;
    /// // This rule will use `BepInEx/plugins` as the target directory and `plugins` as the route name.
    /// let rule = RouteRule::new_static("BepInEx/plugins").with_file_extension("dll");
    ///
    /// assert!(rule.matches("plugins/MyPlugin.dll"));
    /// assert!(rule.matches("MyPlugin.dll"));
    /// // `RouteRule` considers the entire part after the first dot to be the extension.
    /// assert!(!rule.matches("MyPlugin.mm.dll"));
    /// assert!(!rule.matches("other.txt"));
    pub fn with_file_extension(mut self, extension: impl Into<String>) -> Self {
        self.file_extensions.push(extension.into());
        self
    }

    /// Replaces the list of file extensions that will match the rule.
    ///
    /// See [`with_file_extension`](RouteRule::with_file_extension) for more details on extension matching.
    pub fn with_file_extensions(mut self, extensions: Vec<String>) -> Self {
        self.file_extensions = extensions;
        self
    }

    /// Controls whether the prelude (the part of the path before the route name) is preserved in the output path.
    ///
    /// This option is enabled by default.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// # use loadsmith_install::rule::RouteRule;
    /// // Flattening is enabled by default.
    /// let flattening_rule = RouteRule::new_static("BepInEx/plugins");
    /// let non_flattening_rule = RouteRule::new_static("BepInEx/plugins").with_flatten(false);
    ///
    /// let package = PackageRef::new("Author-Name", Version::new(1, 0, 0));
    ///
    /// // The `Nested` folder is removed from the output path when flattening is enabled...
    /// assert_eq!(
    ///     flattening_rule.map_file("Nested/plugins/MyPlugin.dll", &package),
    ///     Some("BepInEx/plugins/Author-Name/MyPlugin.dll".into())
    /// );
    ///
    /// // ... but preserved when flattening is disabled.
    /// assert_eq!(
    ///     non_flattening_rule.map_file("Nested/plugins/MyPlugin.dll", &package),
    ///     Some("BepInEx/plugins/Author-Name/Nested/MyPlugin.dll".into())
    /// );
    ///
    /// // The same goes even if the route name is not present in the path.
    /// assert_eq!(
    ///     flattening_rule.map_file("not-plugins/MyPlugin.dll", &package),
    ///     Some("BepInEx/plugins/Author-Name/MyPlugin.dll".into())
    /// );
    ///
    /// assert_eq!(
    ///     non_flattening_rule.map_file("not-plugins/MyPlugin.dll", &package),
    ///     Some("BepInEx/plugins/Author-Name/not-plugins/MyPlugin.dll".into())
    /// );
    ///
    /// // Intermediate directories after the route name (the remainder) are preserved even when flattening is enabled.
    /// assert_eq!(
    ///     flattening_rule.map_file("plugins/Nested/MyPlugin.dll", &package),
    ///     Some("BepInEx/plugins/Author-Name/Nested/MyPlugin.dll".into())
    /// );
    /// ```
    pub fn with_flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    /// Controls whether a subdirectory named after the package ID is inserted between the target and the remainder of the path.
    ///
    /// This option is enabled by default.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// # use loadsmith_install::rule::RouteRule;
    /// // Subdirectory insertion is enabled by default.
    /// let subdir_rule = RouteRule::new_static("BepInEx/plugins");
    /// let no_subdir_rule = RouteRule::new_static("BepInEx/plugins").with_subdir(false);
    ///
    /// let package = PackageRef::new("Author-Name", Version::new(1, 0, 0));
    ///
    /// // An `Author-Name` directory is inserted with subdirectory insertion enabled...
    /// assert_eq!(
    ///   subdir_rule.map_file("plugins/MyPlugin.dll", &package),
    ///   Some("BepInEx/plugins/Author-Name/MyPlugin.dll".into())
    /// );
    ///
    /// // ... with it disabled, files (and directories) are installed directly into the target directory.
    /// assert_eq!(
    ///   no_subdir_rule.map_file("plugins/MyPlugin.dll", &package),
    ///   Some("BepInEx/plugins/MyPlugin.dll".into())
    /// );
    /// assert_eq!(
    ///   no_subdir_rule.map_file("plugins/Nested/MyPlugin.dll", &package),
    ///   Some("BepInEx/plugins/Nested/MyPlugin.dll".into())
    /// );
    /// ```
    pub fn with_subdir(mut self, subdir: bool) -> Self {
        self.subdir = subdir;
        self
    }

    /// Sets whether the rule excepts files to be mutable (i.e. config files or other user-editable files)
    /// or immutable (i.e. binaries, libraries, etc.).
    ///
    /// This is used to determine whether the rule prefers hard links over file copies. Immutable files are linked,
    /// while mutable files are copied. Note that the actual behavior depends on the implementation of the install operation;
    /// this is just a hint to the installer.
    ///
    /// This option is disabled by default.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use loadsmith_install::rule::RouteRule;
    /// let mutable_rule = RouteRule::new_static("BepInEx/config").with_mutable(true);
    /// let immutable_rule = RouteRule::new_static("BepInEx/plugins").with_mutable(false);
    ///
    /// assert!(!mutable_rule.use_links());
    /// assert!(immutable_rule.use_links());
    /// ```
    pub fn with_mutable(mut self, mutable: bool) -> Self {
        self.mutable = mutable;
        self
    }

    /// Returns `true` if the path matches by extension or by route name.
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.matches_extension(path) || self.matches_path(path)
    }

    /// Returns `true` if the file name has one of the rule's registered extensions.
    ///
    /// Contrary to [`Path::extension`](std::path::Path::extension), this method considers the
    /// entire part after the first dot in the file name to be the extension.
    ///
    /// Extensions can be registered using [`with_file_extension`](RouteRule::with_file_extension) or
    /// [`with_file_extensions`](RouteRule::with_file_extensions).
    pub fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        // check the whole extension, that is everything after the first dot in the file name
        path.as_ref()
            .file_name()
            .and_then(|name| name.split_once('.'))
            .map(|(_, ext)| self.file_extensions.iter().any(|e| e == ext))
            .unwrap_or(false)
    }

    /// Returns `true` if the path contains the route's name as a path component.
    pub fn matches_path(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.split_path(path.as_ref()).is_some()
    }

    fn split_path(&self, path: &Utf8Path) -> Option<(Utf8PathBuf, Utf8PathBuf)> {
        // eat components until we find the route name
        let Some(route_name_index) = path
            .components()
            .position(|comp| comp.as_str().eq_ignore_ascii_case(self.name.as_ref()))
        else {
            // no match
            return None;
        };

        let prefix = path.components().take(route_name_index).collect();
        let suffix = path.components().skip(route_name_index + 1).collect();

        Some((prefix, suffix))
    }

    /// Maps a file path to its final install destination.
    ///
    /// See the [struct-level documentation](RouteRule) for more information on how the mapping works.
    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        let (prefix, suffix) = self.split_path(path).unwrap_or_else(|| {
            let mut components = path.components();
            let file_name = components
                .next_back()
                .map(|file_name| file_name.as_str())
                .unwrap_or_default();
            let prefix = components.collect();

            (prefix, Utf8PathBuf::from(file_name))
        });

        let mut target_path = self.target.clone();

        if self.subdir {
            target_path.push(package.id().as_str());
        }

        if !self.flatten {
            target_path.push(prefix);
        }

        target_path.push(suffix);

        Some(target_path)
    }

    /// Returns `true` if the rule prefers hard links over file copies.
    ///
    /// Links are used when the rule is not marked as mutable.
    pub fn use_links(&self) -> bool {
        !self.mutable
    }

    /// Returns the conflict strategy for this route rule (always [`Skip`](ConflictStrategy::Skip)).
    ///
    /// Route rules never overwrite existing files and never error; they silently skip.
    pub fn conflict_strategy(&self) -> ConflictStrategy {
        ConflictStrategy::Skip
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::{PackageId, Version};

    use super::*;

    #[test]
    fn new() {
        let rule = RouteRule::new(Utf8Path::new("BepInEx/plugins"));

        assert_eq!(rule.name, "plugins");
        assert_eq!(rule.target, Utf8Path::new("BepInEx/plugins"));

        let rule = RouteRule::new(Utf8Path::new("MelonLoader"));

        assert_eq!(rule.name, "MelonLoader");
        assert_eq!(rule.target, Utf8Path::new("MelonLoader"));

        let rule = RouteRule::new(Utf8Path::new(""));

        assert_eq!(rule.name, "");
        assert_eq!(rule.target, Utf8Path::new(""));
    }

    #[test]
    fn defaults() {
        let rule = RouteRule::new_static("BepInEx/plugins");

        assert!(rule.flatten);
        assert!(rule.subdir);
        assert!(!rule.mutable);
    }

    #[test]
    fn matches_path() {
        let rule = RouteRule::new_static("BepInEx/plugins");

        assert!(rule.matches("BepInEx/plugins/myplugin.dll"));
        assert!(rule.matches("plugins/myplugin.dll"));
        assert!(rule.matches("Plugins/myplugin.dll"));
        assert!(rule.matches("Nested/BepInEx/plugins/myplugin.dll"));
        assert!(!rule.matches("BepInEx/core/myplugin.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn matches_extension() {
        let rule = RouteRule::new_static("BepInEx/monomod").with_file_extension("mm.dll");

        assert!(rule.matches("BepInEx/monomod/myplugin.mm.dll"));
        assert!(rule.matches("BepInEx/monomod/myplugin.dll"));
        assert!(rule.matches("myplugin.mm.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn match_extension_with_dot() {
        let rule1 = RouteRule::new_static("plugins").with_file_extension("dll");

        assert!(rule1.matches("myplugin.dll"));
        assert!(!rule1.matches("myplugin.mm.dll"));
        assert!(!rule1.matches("myplugin.dll.mm"));
        assert!(!rule1.matches("myplugin.dll.dll"));

        let rule2 = RouteRule::new_static("monomod").with_file_extension("mm.dll");

        assert!(!rule2.matches("myplugin.dll"));
        assert!(rule2.matches("myplugin.mm.dll"));
        assert!(!rule2.matches("myplugin.mm"));
        assert!(!rule2.matches("myplugin.mm.mm.dll"));
    }

    macro_rules! assert_map {
        ($rule:expr, $file:expr, $package:expr => None) => {
            assert_eq!($rule.map_file($file, $package), None);
        };
        ($rule:expr, $file:expr, $package:expr => $expected:expr) => {
            assert_eq!(
                $rule.map_file($file, $package),
                Some(Utf8PathBuf::from($expected))
            );
        };
    }

    #[test]
    fn map_file() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_file_extension("plugin")
            .with_subdir(true)
            .with_flatten(true);

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        // defaults correctly when name is not present in the path
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // removes the matched "plugins" component
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // removes the matched "pluguins" component and flattens the "BepInEx" component
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // case-insensitive match of "plugins"
        assert_map!(rule, "BepInEx/Plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // flattens the "Nested" component
        assert_map!(rule, "Nested/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // flattens the "Nested" component
        assert_map!(rule, "Nested/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // retains nesting after the matched "plugins" component but flattens before
        assert_map!(rule, "Before/plugins/After/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/After/MyPlugin.plugin");
    }

    #[test]
    fn map_file_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_flatten(true)
            .with_subdir(true)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        // Flattened routing keeps only the file name and package folder.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // When the route name is already present, it is removed from the output path.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // A fully qualified route path still collapses down to the target folder.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // Extra nesting after the route name is preserved even when flattening.
        assert_map!(rule, "plugins/Nested/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/Nested/MyPlugin.dll");

        // File extension matching does not change the mapped directory layout.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/Nested/MyPlugin.plugin");

        // Paths without the route name still map relative to the file name.
        assert_map!(rule, "Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/MyPlugin.plugin");
    }

    #[test]
    fn map_file_no_subdir_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_subdir(false)
            .with_flatten(true)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        // Without subdir support, the package id is never inserted.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // The route prefix is still stripped when it appears in the input path.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Flattening keeps fully qualified paths at the target root.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Nested content stays nested when flattening does not remove it.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Nested/MyPlugin.plugin");

        // Nested content stays nested when flattening does not remove it.
        assert_map!(rule, "BepInEx/plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Nested/MyPlugin.plugin");
    }

    #[test]
    fn map_file_no_subdir_no_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_subdir(false)
            .with_flatten(false)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        // No package folder is inserted when subdir is disabled.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Nested routes are preserved even when the route name doesn't appear.
        assert_map!(rule, "other/MyPlugin.dll", &package => "BepInEx/plugins/other/MyPlugin.dll");

        // A matching route component is removed before the remainder is appended.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Without flattening, the unmatched prefix is preserved in the output path.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/BepInEx/MyPlugin.dll");

        // Nested content remains nested when the route name is stripped.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Nested/MyPlugin.plugin");
    }

    #[test]
    fn map_file_empty() {
        let rule = RouteRule::new_static("BepInEx/plugins").with_flatten(false);

        // An empty path falls back to the package directory when flattening is off.
        assert_map!(
            rule,
            "",
            &PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0)) => "BepInEx/plugins/Author-Name"
        );
    }
}
