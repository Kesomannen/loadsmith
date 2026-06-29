mod error;
mod ops;
mod rule;
mod zip;

pub use error::{Error, Result};
pub use ops::{
    extract::{extract_reader, extract_zip},
    install::{ConflictStrategy, install, uninstall},
};
pub use rule::{GlobRule, InstallRule, InstallRuleset, OwnedInstallRuleset, RouteRule};
