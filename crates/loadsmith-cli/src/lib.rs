mod context;
mod fmt;
mod manifest;
mod profile;

use std::path::PathBuf;

use anyhow::Context as _;
use camino::Utf8Path;
use loadsmith::core::PackageId;
use tracing::{debug, info, warn};
use tracing_indicatif::{span_ext::IndicatifSpanExt, style::ProgressStyle};

use crate::profile::Profile;

pub use context::Context;

// type Error = anyhow::Error;
type Result<T = ()> = anyhow::Result<T>;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose logging")]
    pub verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    #[command(about = "Download and install mods for the current profile")]
    Install,
    #[command(about = "Update mods for the current profile")]
    Update,
    #[command(about = "Update the Thunderstore package index for the current profile's game")]
    Fetch,
    #[command(about = "Read the current profile and report any issues")]
    Check,
    #[command(about = "Launch the game with the current profile")]
    Launch {
        game_path: PathBuf,
    },
    Remove {
        package: PackageId,
    },
}

impl Command {
    fn is_profile_command(&self) -> bool {
        true
    }
}

impl Cli {
    pub async fn run(self, ctx: Context) -> Result {
        if !self.command.is_profile_command() {
            todo!()
        }

        let mut profile = ctx.read_profile().context("failed to read profile")?;

        if !matches!(self.command, Command::Fetch) {
            Self::check_index(&ctx, &profile)
                .await
                .context("failed to check package index")?;
        }

        let res = match self.command {
            Command::Install => Self::install(&ctx, &mut profile).await,
            Command::Update => Self::update(&ctx, &mut profile).await,
            Command::Fetch => Self::fetch(&ctx, &profile).await,
            Command::Check => Self::check(&ctx, &profile).await,
            Command::Launch { game_path } => Self::launch(&ctx, &profile, game_path).await,
            Command::Remove { package } => Self::remove(&ctx, &mut profile, package).await,
        };

        ctx.write_profile(&profile)?;
        res
    }

    async fn check_index(ctx: &Context, profile: &Profile) -> Result {
        const MAX_AGE_BEFORE_WARN: chrono::Duration = chrono::Duration::days(7);

        let metadata = ctx.index.community_metadata(profile.game())?;
        let last_updated = metadata.and_then(|m| m.last_updated);

        match last_updated {
            Some(last_updated) => {
                let age = chrono::Utc::now() - last_updated;

                if age < MAX_AGE_BEFORE_WARN {
                    debug!(
                        %last_updated,
                        "package index is fresh"
                    );
                } else {
                    warn!(
                        %last_updated,
                        "package index is older than 7 days, consider running `loadsmith fetch` to receive the latest mod updates"
                    );
                }
            }
            None => {
                info!(
                    "package index has not been built yet for {}",
                    profile.game()
                );
                Self::fetch(ctx, profile).await?;
            }
        }

        Ok(())
    }

    async fn install(ctx: &Context, profile: &mut Profile) -> Result {
        profile.resolve_and_update_lockfile(ctx, false).await?;
        // profile.sync(ctx).await?;

        Ok(())
    }

    async fn update(ctx: &Context, profile: &mut Profile) -> Result {
        profile.resolve_and_update_lockfile(ctx, true).await?;
        profile.sync(ctx).await?;

        Ok(())
    }

    async fn fetch(ctx: &Context, profile: &Profile) -> Result {
        let span = tracing::info_span!("fetch");
        span.pb_set_style(&ProgressStyle::default_spinner());
        span.pb_set_message("fetching package index...");

        let _enter = span.enter();

        ctx.index.update(profile.game()).await?;

        Ok(())
    }

    async fn check(_ctx: &Context, _profile: &Profile) -> Result {
        info!("all systems go!");

        Ok(())
    }

    async fn launch(ctx: &Context, profile: &Profile, game_path: PathBuf) -> Result {
        let schema = profile.get_schema(ctx).await?;
        let loader = profile.make_loader(&schema)?;

        let profile_path = Utf8Path::from_path(&ctx.working_dir)
            .context("working directory is not valid UTF-8")?;
        let game_path = Utf8Path::from_path(&game_path).context("game path is not valid UTF-8")?;

        let launch_ctx = loadsmith::loader::LaunchContext::new(profile_path, game_path, true);

        let mut command = std::process::Command::new("steam");

        let args = loader
            .get_launch_args(&launch_ctx)
            .context("error generating launch arguments")?;
        args.apply(&mut command);

        loader
            .prepare_launch(&launch_ctx)
            .context("error preparing for launch")?;

        debug!("launching game with command: {:?}", command);

        // command
        //     .spawn()
        //     .context("failed to launch game")?
        //     .wait()
        //     .context("failed to wait for game process")?;

        Ok(())
    }

    async fn remove(ctx: &Context, profile: &mut Profile, package: PackageId) -> Result {
        let (package_id, _) = profile.manifest.mods.get_or_search(&package)?;
        profile.manifest.mods.remove(&package_id.clone());

        profile.resolve_and_update_lockfile(ctx, false).await?;
        profile.sync(ctx).await?;

        Ok(())
    }
}
