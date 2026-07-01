use crate::{Context, Result, manifest::Manifest};

pub struct InstallCommand {}

impl InstallCommand {
    pub async fn run(self, ctx: Context) -> Result {
        let manifest = ctx.read_manifest()?;
        let lockfile = ctx.read_lockfile_option()?;

        Ok(())
    }
}
