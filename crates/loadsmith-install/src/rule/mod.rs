//! Data-driven rules for package installation.
//!
//! The structs in this module answer the question: "Given a file in a package archive, where should it be installed on disk?".
//!
//! An [`InstallRule`] is a single rule that maps files from an in-archive path to an install destination.
//! A rule can decide whether it should be used for a given path with the [`matches`](InstallRule::matches) method.
//! Then, the [`map_file`](InstallRule::matches) method can be used to map the file to its final install destination
//! or, if `None` was returned, skip the file.
//! Note that `map_file` may still be called even if the rule does not match the path, for instance if the rule
//! is defined as the default rule for a ruleset.
//!
//! An [`InstallRuleset`] is a collection of rules that can be used to map files from a package archive to install destinations,
//! with an optional default rule and options to always exclude certain files. It is a borrowed view of a ruleset, whereas
//! [`OwnedInstallRuleset`] is the owned version.
//!
//! Rules operate on a specialized version of file paths called [`Utf8Path`] and [`Utf8PathBuf`].
//! These are guaranteed to be valid UTF-8 and come from the [`camino`] crate.
//!
//! Currently, there are two types of rules: [`GlobRule`] and [`RouteRule`].
//! You can read about their behavior on their respective documentation pages.
//!
//! # Examples
//!
//! Using a single rule:
//!
//! ```
//! # use loadsmith_core::{PackageRef, PackageId, Version};
//! # use loadsmith_install::rule::{InstallRule, GlobRule, RouteRule, InstallRuleset};
//! # use camino::Utf8Path;
//! // Map files with a `dll` extension to the `plugins` directory.
//! let rule = InstallRule::Glob(GlobRule::try_from_pattern("*.dll", "plugins").unwrap());
//!
//! // `map_file` always requires a `PackageRef` for context, in case the rule is configured to separate files by package name.
//! // However in this simple case, our rule doesn't care about the package name or version.
//! let package = PackageRef::new("Author-Name", Version::new(1, 0, 0));
//!
//! assert!(rule.matches("MyPlugin.dll"));
//! assert_eq!(rule.map_file("MyPlugin.dll", &package), Some("plugins/MyPlugin.dll".into()));
//!
//! assert!(!rule.matches("config.txt"));
//! // Even though the rule does not match, it can still be used to map the file.
//! assert_eq!(rule.map_file("config.txt", &package), Some("plugins/config.txt".into()));
//! ```
//!
//! Normal zip extraction logic with a top-level directory stripped:
//!
//! ```
//! # use loadsmith_core::{PackageRef, PackageId, Version};
//! # use loadsmith_install::rule::{InstallRule, GlobRule, RouteRule, InstallRuleset};
//! # use camino::Utf8Path;
//! let rules = vec![
//!     InstallRule::Glob(
//!         GlobRule::try_from_pattern("*", ".")
//!             .unwrap()
//!             .strip_top_level(true)
//!     ),
//! ];
//!
//! let ruleset = InstallRuleset::new(&rules);
//!
//! let pkg = PackageRef::new("Author-Mod", Version::new(1, 0, 0));
//!
//! assert_eq!(
//!     ruleset.map_file("TopLevelFile.txt", &pkg),
//!     None
//! );
//! assert_eq!(
//!     ruleset.map_file("TopLevelDir/File.txt", &pkg),
//!     Some("./File.txt".into())
//! );
//! assert_eq!(
//!     ruleset.map_file("TopLevelDir/Subdir/File.txt", &pkg),
//!     Some("./Subdir/File.txt".into())
//! );
//! ```
//!
//! If you are familiar with Thunderstore's BepInEx plugin installation rules,
//! here is an example of how they're modelled with `InstallRule`s and `InstallRuleset`s:
//!
//! ```
//! # use loadsmith_core::{PackageRef, PackageId, Version};
//! # use loadsmith_install::rule::{InstallRule, GlobRule, RouteRule, InstallRuleset};
//! let rules = vec![
//!     InstallRule::Route(RouteRule::new_static("BepInEx/plugins")),
//!     InstallRule::Route(
//!         RouteRule::new_static("BepInEx/monomod")
//!             .with_file_extension("mm.dll")
//!     ),
//!     InstallRule::Route(RouteRule::new_static("BepInEx/patchers")),
//!     InstallRule::Route(RouteRule::new_static("BepInEx/core")),
//!     InstallRule::Route(
//!         RouteRule::new_static("BepInEx/config")
//!             .with_flatten(true)
//!             .with_subdir(false)
//!             .with_mutable(true)
//!     ),
//! ];
//! let ruleset = InstallRuleset::new(&rules).with_default_rule(rules.first().unwrap());
//!
//! let pkg = PackageRef::new("Author-Mod", Version::new(1, 0, 0));
//!
//! assert_eq!(
//!     ruleset.map_file("plugins/MyPlugin.dll", &pkg),
//!     Some("BepInEx/plugins/Author-Mod/MyPlugin.dll".into())
//! );
//! assert_eq!(
//!     ruleset.map_file("mymonomod.mm.dll", &pkg),
//!     Some("BepInEx/monomod/Author-Mod/mymonomod.mm.dll".into())
//! );
//! assert_eq!(
//!     ruleset.map_file("patchers/MyPatcher.dll", &pkg),
//!     Some("BepInEx/patchers/Author-Mod/MyPatcher.dll".into())
//! );
//! assert_eq!(
//!     ruleset.map_file("core/MyCore.dll", &pkg),
//!     Some("BepInEx/core/Author-Mod/MyCore.dll".into())
//! );
//! assert_eq!(
//!     ruleset.map_file("config/MyPlugin.cfg", &pkg),
//!     Some("BepInEx/config/MyPlugin.cfg".into())
//! );
//! // Matches no rule; falls back to default rule (plugins).
//! assert_eq!(
//!     ruleset.map_file("README.md", &pkg),
//!     Some("BepInEx/plugins/Author-Mod/README.md".into())
//! );
//! ```

use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use globset::GlobSet;
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

use crate::ConflictStrategy;

pub use glob::GlobRule;
pub use route::RouteRule;

mod glob;
mod route;

/// A file-mapping rule that can be either a [`GlobRule`] or a [`RouteRule`].
///
/// The main method of interest is [`map_file`](InstallRule::map_file), which maps a
/// file path to its final install destination with the context of a [`PackageRef`].
/// The method can return `None` if the rule is configured to exclude the file.
///
/// [`matches`](InstallRule::matches) can be used to check if a rule should be applied
/// to a given path, for example if a glob pattern matches, but it is not required to call `map_file`.
///
/// # Examples
///
/// ```
/// # use loadsmith_core::{PackageRef, PackageId, Version};
/// # use loadsmith_install::rule::{InstallRule, GlobRule};
/// // Simple glob rule that maps all .dll files to the "Plugins" directory.
/// let rule = InstallRule::Glob(GlobRule::try_from_pattern("*.dll", "Plugins").unwrap());
///
/// assert!(rule.matches("MyPlugin.dll"));
/// assert!(rule.matches("Nested/MyPlugin.dll"));
/// assert!(!rule.matches("config.txt"));
///
/// let pkg = PackageRef::new("Author-Mod", Version::new(1, 0, 0));
/// assert_eq!(
///     rule.map_file("MyPlugin.dll", &pkg),
///     Some("Plugins/MyPlugin.dll".into())
/// );
/// assert_eq!(
///     rule.map_file("Nested/MyPlugin.dll", &pkg),
///     Some("Plugins/Nested/MyPlugin.dll".into())
/// );
/// // Even files that don't match the rule can be mapped.
/// assert_eq!(
///     rule.map_file("config.txt", &pkg),
///     Some("Plugins/config.txt".into())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InstallRule {
    /// Maps files using a glob pattern. See [`GlobRule`] for more details.
    Glob(GlobRule),
    /// Maps files by matching a route (directory name) and file extension. See [`RouteRule`] for more details.
    Route(RouteRule),
}

/// A borrowed set of install rules used to map and filter package files.
///
/// This is the borrowed version of [`OwnedInstallRuleset`].
///
/// See the [module-level documentation](self) for more details.
#[derive(Debug, Clone, Copy)]
pub struct InstallRuleset<'a> {
    exclude: Option<&'a GlobSet>,
    rules: &'a [InstallRule],
    default_rule: Option<&'a InstallRule>,
}

/// An owned set of install rules with optional exclude glob and default rule.
///
/// Unlike [`InstallRuleset`], this struct owns all its data. Use
/// [`as_ref()`](OwnedInstallRuleset::as_ref) to borrow it as an `InstallRuleset`.
///
/// See the [module-level documentation](self) for more details.
#[derive(Debug, Clone, Default)]
pub struct OwnedInstallRuleset {
    exclude: Option<GlobSet>,
    rules: Vec<InstallRule>,
    default_rule: Option<usize>,
}

impl InstallRule {
    /// Returns `true` if the path matches this rule by path content or file extension.
    ///
    /// Even if this returns `false`, [`map_file`](InstallRule::map_file) may still be called on the rule.
    /// Conversely, even if this returns `true`, [`map_file`](InstallRule::map_file) may still return `None`
    /// if the rule is configured to exclude the file.
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.matches_path(path) || self.matches_extension(path)
    }

    /// Returns `true` if the path matches this rule by path content.
    ///
    /// For [`Glob`](InstallRule::Glob) rules this checks the glob pattern.
    /// For [`Route`](InstallRule::Route) rules this looks for the route name in the path.
    pub fn matches_path(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches_path(path),
        }
    }

    /// Returns `true` if the file extension matches this rule.
    ///
    /// For [`Glob`](InstallRule::Glob) rules this always returns `false`.
    /// For [`Route`](InstallRule::Route) rules this checks the registered extensions.
    pub fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(_) => false,
            InstallRule::Route(route) => route.matches_extension(path),
        }
    }

    /// Maps a file path to its final install destination.
    ///
    /// If this returns `None`, the file should be excluded from installation.
    ///
    /// Some rules may choose to separate files by their package name, so the [`PackageRef`] is provided for context.
    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        match self {
            InstallRule::Glob(glob) => glob.map_file(path, package),
            InstallRule::Route(route) => route.map_file(path, package),
        }
    }

    /// Returns `true` if the rule prefers hard links over file copies.
    ///
    /// If this returns `false`, it usually means the rule expects files in the destination to be mutable,
    /// but this is not guaranteed.
    pub fn use_links(&self) -> bool {
        match self {
            InstallRule::Glob(glob) => glob.use_links(),
            InstallRule::Route(route) => route.use_links(),
        }
    }

    /// Returns the conflict strategy for this rule.
    ///
    /// [`Glob`](InstallRule::Glob) rules always use [`Overwrite`](ConflictStrategy::Overwrite).
    /// [`Route`](InstallRule::Route) rules always use [`Skip`](ConflictStrategy::Skip).
    pub fn conflict_strategy(&self) -> ConflictStrategy {
        match self {
            InstallRule::Glob(_) => ConflictStrategy::Overwrite,
            InstallRule::Route(route) => route.conflict_strategy(),
        }
    }
}

impl From<GlobRule> for InstallRule {
    fn from(glob: GlobRule) -> Self {
        InstallRule::Glob(glob)
    }
}

impl From<RouteRule> for InstallRule {
    fn from(route: RouteRule) -> Self {
        InstallRule::Route(route)
    }
}

impl<'a> InstallRuleset<'a> {
    /// Creates a new `InstallRuleset` from a slice of rules.
    pub const fn new(rules: &'a [InstallRule]) -> Self {
        Self {
            rules,
            default_rule: None,
            exclude: None,
        }
    }

    /// Sets the default rule used when no other rule matches a file.
    pub fn with_default_rule(mut self, default_rule: &'a InstallRule) -> Self {
        self.default_rule = Some(default_rule);
        self
    }

    /// Sets a glob set for excluding files from installation.
    pub fn with_exclude(mut self, exclude: &'a GlobSet) -> Self {
        self.exclude = Some(exclude);
        self
    }

    /// Returns the list of install rules.
    pub fn rules(&self) -> &[InstallRule] {
        self.rules
    }

    /// Returns the default rule, if any.
    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule
    }

    /// Returns the exclude glob set, if any.
    pub fn exclude(&self) -> Option<&GlobSet> {
        self.exclude
    }

    /// Finds the first rule that matches the given path, falling back to the
    /// default rule if no path or extension match is found.
    ///
    /// Path-based matches are prioritized before extension-based matches.
    /// Excludes are not considered; use [`is_excluded`](InstallRuleset::is_excluded) to check for excludes first.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, PackageId, Version};
    /// # use loadsmith_install::rule::{InstallRule, GlobRule, RouteRule, InstallRuleset};
    /// let rules = vec![
    ///     InstallRule::Route(RouteRule::new_static("Mods").with_file_extension("dll")),
    ///     InstallRule::Route(RouteRule::new_static("Plugins")),
    /// ];
    /// let ruleset = InstallRuleset::new(&rules).with_default_rule(&rules[0]);
    ///
    /// let pkg = PackageRef::new("Author-Name", Version::new(1, 0, 0));
    ///
    /// assert_eq!(
    ///    ruleset.find_rule_for_path("Plugins/image.png"),
    ///     Some(&rules[1])
    /// );
    /// // No rule matches; the default rule is returned.
    /// assert_eq!(
    ///    ruleset.find_rule_for_path("OtherFile.txt"),
    ///    Some(&rules[0])
    /// );
    /// // The file extension matches the "Mods" rule, so it is returned.
    /// assert_eq!(
    ///    ruleset.find_rule_for_path("Other.dll"),
    ///    Some(&rules[0])
    /// );
    /// // Path-based match takes precedence over extension-based match, so the "Plugins" rule is returned.
    /// assert_eq!(
    ///     ruleset.find_rule_for_path("Plugins/Plugin.dll"),
    ///     Some(&rules[1])
    /// );
    /// ```
    pub fn find_rule_for_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules
            .iter()
            .find(|rule| rule.matches_path(path))
            .or_else(|| self.rules.iter().find(|rule| rule.matches_extension(path)))
            .or(self.default_rule)
    }

    /// Maps a file path through the ruleset, returning the install destination.
    ///
    /// This is equivalent to calling [`map_file_and_return_rule`](InstallRuleset::map_file_and_return_rule) and discarding the rule.
    ///
    /// Returns `None` if the file is excluded or no rule matches (and no default is configured).
    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        self.map_file_and_return_rule(path, package)
            .map(|(mapped, _rule)| mapped)
    }

    /// Maps a file path and returns the install destination together with the
    /// matching rule.
    ///
    /// This is equivalent to calling [`find_rule_for_path`](InstallRuleset::find_rule_for_path) and then calling [`map_file`](InstallRule::map_file) on the rule,
    /// wit the caveat that excluded files will return `None` instead of the default rule.
    ///
    /// Returns `None` if the file is excluded or no rule matches (and no default is configured).
    pub fn map_file_and_return_rule(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<(Utf8PathBuf, &InstallRule)> {
        let path = path.as_ref();
        if self.is_excluded(path) {
            return None;
        }

        self.find_rule_for_path(path)
            .and_then(|rule| rule.map_file(path, package).map(|path| (path, rule)))
    }

    /// Returns `true` if the path matches the exclude glob set and should not be installed.
    ///
    /// This is automatically checked by [`map_file`](InstallRuleset::map_file) and [`map_file_and_return_rule`](InstallRuleset::map_file_and_return_rule).
    pub fn is_excluded(&self, path: impl AsRef<Path>) -> bool {
        self.exclude
            .as_ref()
            .map_or(false, |exclude| exclude.is_match(path))
    }
}

impl OwnedInstallRuleset {
    /// Creates an `OwnedInstallRuleset` from an iterator of items that convert
    /// into [`InstallRule`].
    ///
    /// Returns `None` if `default_rule_index` is out of bounds.
    pub fn from_rule_iter<I, R>(rules: I, default_rule_index: Option<usize>) -> Option<Self>
    where
        I: IntoIterator<Item = R>,
        R: Into<InstallRule>,
    {
        let rules = rules.into_iter().map(Into::into).collect();
        Self::with_rules(rules, default_rule_index)
    }

    /// Creates an [`OwnedInstallRuleset`] from a vector of rules.
    ///
    /// Returns `None` if `default_rule_index` is out of bounds.
    pub fn with_rules(rules: Vec<InstallRule>, default_rule_index: Option<usize>) -> Option<Self> {
        if default_rule_index.is_some_and(|index| !(0..rules.len()).contains(&index)) {
            return None;
        }

        Some(Self {
            rules,
            default_rule: default_rule_index,
            exclude: None,
        })
    }

    /// Sets an exclude glob set.
    pub fn with_exclude(mut self, exclude: GlobSet) -> Self {
        self.exclude = Some(exclude);
        self
    }

    /// Appends a rule to the end of the rule list.
    pub fn add_rule(&mut self, rule: InstallRule) {
        self.rules.push(rule);
    }

    /// Inserts a rule at the given index, adjusting the default rule index if needed.
    pub fn insert_rule(&mut self, index: usize, rule: InstallRule) {
        if let Some(default_index) = self.default_rule {
            if index <= default_index {
                self.default_rule = Some(default_index + 1);
            }
        }

        self.rules.insert(index, rule);
    }

    /// Replaces or sets the exclude glob set.
    pub fn set_exclude(&mut self, exclude: GlobSet) {
        self.exclude = Some(exclude);
    }

    /// Returns a reference to the list of rules.
    pub fn rules(&self) -> &[InstallRule] {
        &self.rules
    }

    /// Returns the default rule, if any.
    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule.map(|index| &self.rules[index])
    }

    /// Returns the exclude glob set, if any.
    pub fn exclude(&self) -> Option<&GlobSet> {
        self.exclude.as_ref()
    }

    /// Consumes the ruleset and returns the inner parts:
    /// `(rules, default_rule_index, exclude)`.
    pub fn into_parts(self) -> (Vec<InstallRule>, Option<usize>, Option<GlobSet>) {
        (self.rules, self.default_rule, self.exclude)
    }

    /// Borrows this owned ruleset as an [`InstallRuleset`].
    pub fn as_ref(&self) -> InstallRuleset<'_> {
        InstallRuleset {
            rules: &self.rules,
            default_rule: self.default_rule.map(|index| &self.rules[index]),
            exclude: self.exclude.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::Version;

    use super::*;

    #[test]
    fn ruleset_matches_path_over_extension() {
        let rules = vec![
            InstallRule::Route(RouteRule::new_static("Mods").with_file_extension("dll")),
            InstallRule::Route(RouteRule::new_static("Plugins")),
        ];

        let ruleset = InstallRuleset::new(&rules);

        let pkg = PackageRef::new("test".to_string(), Version::new(1, 0, 0));

        // The path takes precedence over file extension, so the "Plugins" rule
        // should match even though the file has a .dll extension.
        assert_eq!(
            ruleset.map_file("Plugins/Plugin.dll", &pkg),
            Some(Utf8PathBuf::from("Plugins/test/Plugin.dll"))
        );
    }
}
