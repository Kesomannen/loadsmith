use camino::{Utf8Path, Utf8PathBuf};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallRuleset<'a> {
    pub rules: &'a [InstallRule],
    pub default_rule: Option<&'a InstallRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedInstallRuleset {
    rules: Vec<InstallRule>,
    default_rule: Option<usize>,
}

impl InstallRule {
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches(path),
        }
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        match self {
            InstallRule::Glob(glob) => Some(glob.map_file(path, package)),
            InstallRule::Route(route) => route.map_file(path, package),
        }
    }

    pub fn matches_mapped(&self, path: impl AsRef<Utf8Path>) -> bool {
        match self {
            InstallRule::Glob(glob) => path.as_ref().starts_with(&*glob.target),
            InstallRule::Route(route) => path.as_ref().starts_with(&*route.target),
        }
    }

    pub fn use_links(&self) -> bool {
        match self {
            InstallRule::Glob(_) => false,
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
    pub const fn new(rules: &'a [InstallRule], default_rule: Option<&'a InstallRule>) -> Self {
        Self {
            rules,
            default_rule,
        }
    }

    pub fn find_rule_for_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules
            .into_iter()
            .find(|rule| rule.matches(path))
            .or(self.default_rule)
    }

    pub fn find_rule_for_mapped_path(&self, path: impl AsRef<Utf8Path>) -> Option<&'a InstallRule> {
        let path = path.as_ref();
        self.rules
            .into_iter()
            .find(|rule| rule.matches_mapped(path))
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        self.find_rule_for_path(&path)
            .and_then(|rule| rule.map_file(path, package))
    }
}

impl OwnedInstallRuleset {
    pub fn with_rules(rules: Vec<InstallRule>, default_rule_index: Option<usize>) -> Option<Self> {
        if default_rule_index.is_some_and(|index| !(0..rules.len()).contains(&index)) {
            return None;
        }

        Some(Self {
            rules,
            default_rule: default_rule_index,
        })
    }

    pub fn add(&mut self, rule: InstallRule) {
        self.rules.push(rule);
    }

    pub fn rules(&self) -> &[InstallRule] {
        &self.rules
    }

    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule.map(|index| &self.rules[index])
    }

    pub fn as_ref(&self) -> InstallRuleset<'_> {
        InstallRuleset::new(&self.rules, self.default_rule())
    }
}

impl Default for OwnedInstallRuleset {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_rule: None,
        }
    }
}
