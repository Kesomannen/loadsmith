/// Creates a [`globset::Glob`] from a string literal, panicking if the
/// pattern is invalid.
///
/// Intended for use with constant patterns where an invalid glob is a bug.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::glob;
///
/// let g = glob!("version.dll");
/// let matcher = g.compile_matcher();
/// assert!(matcher.is_match("version.dll"));
/// assert!(!matcher.is_match("winmm.dll"));
/// ```
#[macro_export]
macro_rules! glob {
    ($pattern:expr) => {
        globset::Glob::new($pattern).expect("constant pattern should be valid")
    };
}

/// Creates a [`loadsmith_install::GlobRule`] from a pattern and destination
/// path literal, panicking if the pattern is invalid.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::glob_rule;
/// use loadsmith_install::GlobRule;
///
/// let rule: GlobRule = glob_rule!("*.dll" => "plugins");
/// assert!(rule.matches("mod.dll"));
/// assert!(!rule.matches("mod.txt"));
/// ```
#[macro_export]
macro_rules! glob_rule {
    ($pattern:expr => $destination:expr) => {
        loadsmith_install::rule::GlobRule::new(
            $crate::glob!($pattern),
            camino::Utf8Path::new($destination),
        )
    };
}
