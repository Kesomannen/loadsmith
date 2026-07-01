mod command;
mod context;
mod lockfile;
mod manifest;

pub use context::Context;

type Error = anyhow::Error;
type Result<T = ()> = anyhow::Result<T>;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Install {},
}

impl Cli {
    pub async fn run(self, ctx: Context) -> Result {
        match self.command {
            Command::Install {} => {
                command::InstallCommand {}.run(ctx).await?;
            }
        }

        Ok(())
    }
}
