use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, bail};
use camino::Utf8Path;
use loadsmith::{
    core::{PackageId, VersionRange},
    thunderstore::{PackageIdExt, r2z},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use tracing_indicatif::{span_ext::IndicatifSpanExt, style::ProgressStyle};
use uuid::Uuid;

use crate::{
    manifest::{Manifest, Mods, ProfileInfo},
    profile::Profile,
};

mod context;
mod fmt;
mod manifest;
mod profile;
mod util;

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
    Import {
        code: String,
        path: Option<PathBuf>,
    },
    Export,
}

impl Cli {
    pub async fn run(self, ctx: Context) -> Result {
        let ctx = Arc::new(ctx);

        match &self.command {
            Command::Import { code, path } => {
                return Self::import(ctx, code, path.as_deref()).await;
            }
            _ => (),
        };

        let mut profile = Profile::read(&ctx.working_dir).context("failed to read profile")?;

        if !matches!(self.command, Command::Fetch) {
            Self::check_index(&ctx, &profile)
                .await
                .context("failed to check package index")?;
        }

        let res = match self.command {
            Command::Install => Self::install(ctx, &mut profile).await,
            Command::Update => Self::update(ctx, &mut profile).await,
            Command::Fetch => Self::fetch(&ctx, &profile).await,
            Command::Check => Self::check(&ctx, &profile).await,
            Command::Launch { game_path } => Self::launch(&ctx, &profile, game_path).await,
            Command::Remove { package } => Self::remove(ctx, &mut profile, package).await,
            Command::Export => Self::export(&ctx, &profile).await,
            _ => unreachable!(),
        };

        profile.write()?;
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

    async fn install(ctx: Arc<Context>, profile: &mut Profile) -> Result {
        profile.resolve_and_update_lockfile(&ctx, false).await?;
        profile.sync(ctx).await?;

        Ok(())
    }

    async fn update(ctx: Arc<Context>, profile: &mut Profile) -> Result {
        profile.resolve_and_update_lockfile(&ctx, true).await?;
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

    async fn remove(ctx: Arc<Context>, profile: &mut Profile, package: PackageId) -> Result {
        let (package_id, _) = profile.manifest.mods.get_or_search(&package)?;
        profile.manifest.mods.remove(&package_id.clone());

        profile.resolve_and_update_lockfile(&ctx, false).await?;
        profile.sync(ctx).await?;

        Ok(())
    }

    async fn import(ctx: Arc<Context>, key: &str, path: Option<&Path>) -> Result {
        let key: Uuid = key.parse().context("invalid UUID")?;
        let content = {
            let span = tracing::info_span!("import", %key);
            span.pb_set_style(&ProgressStyle::default_spinner());
            span.pb_set_message("fetching profile from Thunderstore...");

            let _enter = span.enter();

            ctx.thunderstore.get_profile(key).await
        }?;

        let mut import = r2z::ImportFile::open(Cursor::new(content))?;
        let import_manifest = import
            .read_manifest::<ExtraImportData>()
            .context("failed to read manifest")?;

        let path = path.map_or_else(
            || ctx.working_dir.join(&import_manifest.profile_name),
            |p| p.to_path_buf(),
        );

        if path.exists() {
            bail!(
                "target path {} already exists, please specify a different path",
                path.display()
            );
        }

        import
            .import_config_files(&path, true)
            .context("failed to import config files")?;

        let game = import_manifest.extra.community.unwrap_or_else(|| {
            warn!("imported profile does not specify a game, defaulting to 'unknown'");

            "unknown".to_string()
        });

        let mods = import_manifest
            .mods
            .into_iter()
            .map(|m| {
                (
                    PackageId::from_ts_ident(m.name),
                    manifest::Mod::Simple(VersionRange::Exact(m.version.into())),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let mut profile =
            Profile::create(path, Manifest::new(ProfileInfo::new(game), Mods::new(mods)));

        let mut res = profile.resolve_and_update_lockfile(&ctx, false).await;
        if res.is_ok() {
            res = profile.sync(ctx).await;
        }

        profile.write()?;
        res
    }

    async fn export(ctx: &Context, profile: &Profile) -> Result {
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ExtraImportData {
    community: Option<String>,
}
