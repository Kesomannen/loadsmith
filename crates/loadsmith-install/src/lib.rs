mod error;
mod ops;
mod rule;
mod zip;

pub use error::{Error, Result};
pub use ops::{
    extract::{extract, extract_from_reader},
    install::{ConflictStrategy, install, uninstall},
};
pub use rule::{GlobRule, InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};
