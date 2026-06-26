use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobBuilder};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobRule {
    pattern: Glob,
    pub(super) target: Cow<'static, Utf8Path>,
    strip_top_level: bool,
}

impl GlobRule {
    pub fn new(pattern: Glob, target: impl Into<Cow<'static, Utf8Path>>) -> Self {
        Self {
            pattern,
            target: target.into(),
            strip_top_level: false,
        }
    }

    pub fn with_strip_top_level(mut self, strip_top_level: bool) -> Self {
        self.strip_top_level = strip_top_level;
        self
    }

    pub fn try_from_pattern(
        pattern: impl AsRef<str>,
        target: impl Into<Cow<'static, Utf8Path>>,
    ) -> Result<Self, globset::Error> {
        let pattern = GlobBuilder::new(pattern.as_ref()).build()?;
        Ok(Self::new(pattern, target))
    }

    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.pattern.compile_matcher().is_match(path.as_ref())
    }

    pub fn map_file(&self, path: impl AsRef<Utf8Path>, _package: &PackageRef) -> Utf8PathBuf {
        let path = path.as_ref();

        let suffix = {
            let mut components = path.components();
            if self.strip_top_level {
                components.next();
            }

            if components.clone().next().is_none() {
                path
            } else {
                components.as_path()
            }
        };

        self.target.as_ref().join(suffix)
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageId;

    use super::*;

    #[test]
    fn matches() {
        let rule = GlobRule::new(
            Glob::new("*.rs").unwrap(),
            Cow::Borrowed(Utf8Path::new("target")),
        );

        assert!(rule.matches("main.rs"));
        assert!(rule.matches("src/main.rs"));
        assert!(!rule.matches("main.txt"));
        assert!(!rule.matches("src/main.txt"));
    }

    #[test]
    fn map_file() {
        let rule = GlobRule::try_from_pattern("*.rs", Utf8Path::new("target")).unwrap();

        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

        assert_eq!(
            rule.map_file("main.rs", &package),
            Utf8PathBuf::from("target/main.rs")
        );

        assert_eq!(
            rule.map_file("src/main.rs", &package),
            Utf8PathBuf::from("target/src/main.rs")
        );

        assert_eq!(
            rule.map_file("main.txt", &package),
            Utf8PathBuf::from("target/main.txt")
        );
    }

    #[test]
    fn map_file_strip_top() {
        let rule = GlobRule::try_from_pattern("in/**", Utf8Path::new("out"))
            .unwrap()
            .with_strip_top_level(true);
        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

        assert_eq!(
            rule.map_file("in/file.txt", &package),
            Utf8PathBuf::from("out/file.txt")
        );

        assert_eq!(
            rule.map_file("in/nested/file.txt", &package),
            Utf8PathBuf::from("out/nested/file.txt")
        );

        assert_eq!(
            rule.map_file("in/nested/deep/file.txt", &package),
            Utf8PathBuf::from("out/nested/deep/file.txt")
        );

        assert_eq!(
            rule.map_file("other.txt", &package),
            Utf8PathBuf::from("out/other.txt")
        );
    }
}
