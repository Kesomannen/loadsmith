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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallRule {
    Glob(GlobRule),
    Route(RouteRule),
}

#[derive(Debug, Clone, Copy)]
pub struct InstallRuleset<'a> {
    exclude: Option<&'a GlobSet>,
    rules: &'a [InstallRule],
    default_rule: Option<&'a InstallRule>,
}

#[derive(Debug, Clone, Default)]
pub struct OwnedInstallRuleset {
    exclude: Option<GlobSet>,
    rules: Vec<InstallRule>,
    default_rule: Option<usize>,
}

impl InstallRule {
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.matches_path(path) || self.matches_extension(path)
    }

    pub fn matches_path(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches_path(path),
        }
    }

    pub fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches_extension(path),
        }
    }

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

    pub fn matches_mapped(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        match self {
            InstallRule::Glob(glob) => path.starts_with(&*glob.target),
            InstallRule::Route(route) => path.starts_with(&*route.target),
        }
    }

    pub fn use_links(&self) -> bool {
        match self {
            InstallRule::Glob(glob) => glob.use_links,
            InstallRule::Route(route) => route.use_links(),
        }
    }

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
    pub const fn new(rules: &'a [InstallRule]) -> Self {
        Self {
            rules,
            default_rule: None,
            exclude: None,
        }
    }

    pub fn with_default_rule(mut self, default_rule: &'a InstallRule) -> Self {
        self.default_rule = Some(default_rule);
        self
    }

    pub fn with_exclude(mut self, exclude: &'a GlobSet) -> Self {
        self.exclude = Some(exclude);
        self
    }

    pub fn rules(&self) -> &[InstallRule] {
        self.rules
    }

    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule
    }

    pub fn exclude(&self) -> Option<&GlobSet> {
        self.exclude
    }

    pub fn find_rule_for_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules
            .iter()
            .find(|rule| rule.matches_path(path))
            .or_else(|| self.rules.iter().find(|rule| rule.matches_extension(path)))
            .or(self.default_rule)
    }

    pub fn find_rule_for_mapped_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules.iter().find(|rule| rule.matches_mapped(path))
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        if self.is_excluded(path) {
            return None;
        }

        self.find_rule_for_path(path)
            .and_then(|rule| rule.map_file(path, package))
    }

    pub fn is_excluded(&self, path: impl AsRef<Path>) -> bool {
        self.exclude
            .as_ref()
            .map_or(false, |exclude| exclude.is_match(path))
    }
}

impl OwnedInstallRuleset {
    pub fn from_rule_iter<I, R>(rules: I, default_rule_index: Option<usize>) -> Option<Self>
    where
        I: IntoIterator<Item = R>,
        R: Into<InstallRule>,
    {
        let rules = rules.into_iter().map(Into::into).collect();
        Self::with_rules(rules, default_rule_index)
    }

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

    pub fn with_exclude(mut self, exclude: GlobSet) -> Self {
        self.exclude = Some(exclude);
        self
    }

    pub fn add_rule(&mut self, rule: InstallRule) {
        self.rules.push(rule);
    }

    pub fn insert_rule(&mut self, index: usize, rule: InstallRule) {
        if let Some(default_index) = self.default_rule {
            if index <= default_index {
                self.default_rule = Some(default_index + 1);
            }
        }

        self.rules.insert(index, rule);
    }

    pub fn set_exclude(&mut self, exclude: GlobSet) {
        self.exclude = Some(exclude);
    }

    pub fn rules(&self) -> &[InstallRule] {
        &self.rules
    }

    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule.map(|index| &self.rules[index])
    }

    pub fn exclude(&self) -> Option<&GlobSet> {
        self.exclude.as_ref()
    }

    pub fn into_parts(self) -> (Vec<InstallRule>, Option<usize>, Option<GlobSet>) {
        (self.rules, self.default_rule, self.exclude)
    }

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
    use super::*;

    #[test]
    fn ruleset_matches_path_over_extension() {
        let rules = vec![
            InstallRule::Route(RouteRule::new_static("Mods").with_file_extension("dll")),
            InstallRule::Route(RouteRule::new_static("Plugins")),
        ];

        let ruleset = InstallRuleset::new(&rules);

        let pkg = PackageRef::new("test".to_string(), (1, 0, 0));

        // The path takes precedence over file extension, so the "Plugins" rule
        // should match even though the file has a .dll extension.
        assert_eq!(
            ruleset.map_file("Plugins/Plugin.dll", &pkg),
            Some(Utf8PathBuf::from("Plugins/test/Plugin.dll"))
        );
    }
}
