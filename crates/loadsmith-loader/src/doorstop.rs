use std::fs;

use tracing::{info, warn};

use crate::{Error, LaunchContext, Result};

const FALLBACK_VERSION: u32 = 3;

pub fn args(
    version_override: Option<u32>,
    ctx: &LaunchContext,
) -> Result<(&'static str, &'static str)> {
    let version = if let Some(v) = version_override {
        info!("using doorstop version override: {}", v);
        v
    } else {
        let path = ctx.profile.join(".doorstop_version");

        if path.exists() {
            let version_content = fs::read_to_string(&path)?;
            let version = version_content
                .split('.') // read only the major version number
                .next()
                .and_then(|str| str.parse().ok())
                .ok_or_else(|| Error::InvalidDoorstopVersionFormat(version_content))?;

            info!(version, "doorstop version read",);
            version
        } else {
            warn!(
                fallback = FALLBACK_VERSION,
                ".doorstop_version file is missing"
            );
            FALLBACK_VERSION
        }
    };

    match version {
        3 => Ok(("--doorstop-enable", "--doorstop-target")),
        4 => Ok(("--doorstop-enabled", "--doorstop-target-assembly")),
        v => Err(Error::UnsupportedDoorstopVersion(v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_default() {
        let tempdir = tempfile::tempdir().unwrap();
        let ctx = LaunchContext::in_tempdir(&tempdir, false).unwrap();

        let (enable, target) = args(None, &ctx).unwrap();
        assert_eq!(enable, "--doorstop-enable");
        assert_eq!(target, "--doorstop-target");
    }

    #[test]
    fn args_version_override() {
        let tempdir = tempfile::tempdir().unwrap();
        let ctx = LaunchContext::in_tempdir(&tempdir, false).unwrap();

        let (enable, target) = args(Some(4), &ctx).unwrap();
        assert_eq!(enable, "--doorstop-enabled");
        assert_eq!(target, "--doorstop-target-assembly");
    }

    #[test]
    fn args_invalid_version_override() {
        let tempdir = tempfile::tempdir().unwrap();
        let ctx = LaunchContext::in_tempdir(&tempdir, false).unwrap();

        let err = args(Some(5), &ctx).unwrap_err();
        assert!(matches!(err, Error::UnsupportedDoorstopVersion(5)));
    }

    #[test]
    fn args_version_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let ctx = LaunchContext::in_tempdir(&tempdir, false).unwrap();

        let version_path = ctx.profile.join(".doorstop_version");
        std::fs::write(&version_path, "4.0.0").unwrap();

        let (enable, target) = args(None, &ctx).unwrap();
        assert_eq!(enable, "--doorstop-enabled");
        assert_eq!(target, "--doorstop-target-assembly");
    }

    #[test]
    fn args_invalid_version_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let ctx = LaunchContext::in_tempdir(&tempdir, false).unwrap();

        let version_path = ctx.profile.join(".doorstop_version");
        std::fs::write(&version_path, "invalid").unwrap();

        let err = args(None, &ctx).unwrap_err();
        assert!(matches!(err, Error::InvalidDoorstopVersionFormat(_)));
    }
}
