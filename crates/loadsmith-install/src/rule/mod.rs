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
/// # Examples
///
/// ```rust
/// use camino::Utf8Path;
/// use loadsmith_install::{InstallRule, GlobRule, RouteRule};
///
/// let glob: InstallRule = GlobRule::try_from_pattern("*.dll", Utf8Path::new("BepInEx/plugins")).unwrap().into();
/// let route: InstallRule = RouteRule::new_static("plugins").into();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallRule {
    /// Maps files using a glob pattern.
    Glob(GlobRule),
    /// Maps files by matching a route (directory name) and file extension.
    Route(RouteRule),
}

/// A borrowed set of install rules used to map and filter package files.
///
/// `InstallRuleset` holds references to an existing rules slice, an optional
/// exclude glob set, and an optional default rule. Use [`OwnedInstallRuleset`]
/// when you need owned storage.
///
/// # Examples
///
/// ```rust
/// use camino::{Utf8Path, Utf8PathBuf};
/// use loadsmith_core::{PackageRef, PackageId, Version};
/// use loadsmith_install::{InstallRuleset, InstallRule, GlobRule};
///
/// let rules = [
///     InstallRule::Glob(GlobRule::try_from_pattern("*.dll", Utf8Path::new("BepInEx/plugins")).unwrap()),
/// ];
/// let ruleset = InstallRuleset::new(&rules);
/// let pkg = PackageRef::new(PackageId::new("x753-More_Suits"), Version::new(1, 0, 3));
///
/// assert_eq!(
///     ruleset.map_file("MyPlugin.dll", &pkg),
///     Some(Utf8PathBuf::from("BepInEx/plugins/MyPlugin.dll"))
/// );
/// ```
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
/// # Examples
///
/// ```rust
/// use camino::Utf8Path;
/// use loadsmith_install::{OwnedInstallRuleset, InstallRule, GlobRule};
///
/// let rules: Vec<InstallRule> = vec![
///     GlobRule::try_from_pattern("*.dll", Utf8Path::new("BepInEx/plugins")).unwrap().into(),
/// ];
/// let owned = OwnedInstallRuleset::with_rules(rules, None).unwrap();
/// let borrowed = owned.as_ref();
/// assert_eq!(borrowed.rules().len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OwnedInstallRuleset {
    exclude: Option<GlobSet>,
    rules: Vec<InstallRule>,
    default_rule: Option<usize>,
}

impl InstallRule {
    /// Returns `true` if the path matches this rule by path content or file extension.
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.matches_path(path) || self.matches_extension(path)
    }

    /// Returns `true` if the path matches this rule by path content.
    ///
    /// For [`Glob`](InstallRule::Glob) rules this checks the glob pattern. For
    /// [`Route`](InstallRule::Route) rules this looks for the route name in the path.
    pub fn matches_path(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches_path(path),
        }
    }

    /// Returns `true` if the file extension matches this rule.
    ///
    /// For [`Glob`](InstallRule::Glob) rules this is identical to `matches_path`.
    /// For [`Route`](InstallRule::Route) rules this checks the registered extensions.
    pub fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches_extension(path),
        }
    }

    /// Maps a file path to its install destination.
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

    /// Returns `true` if the mapped path starts with this rule's target directory.
    pub fn matches_mapped(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => path.starts_with(&*glob.target),
            InstallRule::Route(route) => path.starts_with(&*route.target),
        }
    }

    /// Returns `true` if the rule prefers hard links over file copies.
    pub fn use_links(&self) -> bool {
        match self {
            InstallRule::Glob(glob) => glob.use_links,
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
    /// Path-based matches are checked before extension-based matches.
    pub fn find_rule_for_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules
            .iter()
            .find(|rule| rule.matches_path(path))
            .or_else(|| self.rules.iter().find(|rule| rule.matches_extension(path)))
            .or(self.default_rule)
    }

    /// Finds the first rule whose mapped output path starts with the given path.
    pub fn find_rule_for_mapped_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules.iter().find(|rule| rule.matches_mapped(path))
    }

    /// Maps a file path through the ruleset, returning the install destination.
    ///
    /// Returns `None` if the file is excluded or no rule matches.
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
    /// Returns `None` if the file is excluded or no rule matches.
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

    /// Returns `true` if the path matches the exclude glob set.
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

    /// Creates an `OwnedInstallRuleset` from a vector of rules.
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

    /// Sets an exclude glob set using builder pattern.
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
