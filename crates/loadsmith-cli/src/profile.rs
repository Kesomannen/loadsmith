use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context as _;
use futures::TryStreamExt;
use loadsmith::{
    core::PackageRef,
    install::InstallRuleset,
    loader::Loader,
    manifest::{LockedPackage, Lockfile, ProfileState, ProfileStateData},
};
use thunderstore::models::schema::Schema as ThunderstoreSchema;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use tracing_indicatif::{span_ext::IndicatifSpanExt, style::ProgressStyle};

use crate::{Context, Result, manifest::Manifest, util};

#[derive(Debug)]
pub struct Profile {
    pub manifest: Manifest,
    pub lockfile: Lockfile,
    pub state: ProfileState,
}

impl Profile {
    pub fn new(manifest: Manifest, lockfile: Lockfile, state: ProfileState) -> Self {
        Self {
            manifest,
            lockfile,
            state,
        }
    }

    pub fn create(path: impl Into<PathBuf>, manifest: Manifest) -> Self {
        let lockfile = Lockfile::default();

        let path = path.into();
        let state = ProfileState::new(path, ProfileStateData::default());

        Self {
            manifest,
            lockfile,
            state,
        }
    }

    const MANIFEST_FILE_NAME: &str = "loadsmith.toml";
    const LOCKFILE_FILE_NAME: &str = "loadsmith.lock";
    const PROFILE_STATE_FILE_NAME: &str = "_state/profile.json";

    pub fn read(path: impl Into<PathBuf>) -> Result<Profile> {
        let path = path.into();

        let manifest = Self::read_manifest(&path).context("failed to read manifest")?;
        let lockfile = Self::read_lockfile(&path).context("failed to read lockfile")?;
        let state = Self::read_profile_state(path).context("failed to read profile state")?;

        Ok(Self::new(manifest, lockfile, state))
    }

    fn read_manifest(path: &Path) -> Result<Manifest> {
        util::read_toml(path.join(Self::MANIFEST_FILE_NAME))
    }

    fn read_lockfile(path: &Path) -> Result<Lockfile> {
        let path = path.join(Self::LOCKFILE_FILE_NAME);
        if path.exists() {
            util::read_json(&path)
        } else {
            Ok(Lockfile::default())
        }
    }

    fn read_profile_state(path: PathBuf) -> Result<ProfileState> {
        let state_path = path.join(Self::PROFILE_STATE_FILE_NAME);
        let data = if state_path.exists() {
            util::read_json(&state_path)?
        } else {
            ProfileStateData::default()
        };

        Ok(ProfileState::new(path, data))
    }

    pub fn write(&self) -> Result {
        self.write_manifest()?;
        self.write_lockfile()?;
        self.write_state()?;
        Ok(())
    }

    fn write_manifest(&self) -> Result {
        util::write_toml(self.path().join(Self::MANIFEST_FILE_NAME), &self.manifest)
            .context("failed to write manifest")
    }

    fn write_lockfile(&self) -> Result {
        util::write_json(self.path().join(Self::LOCKFILE_FILE_NAME), &self.lockfile)
            .context("failed to write lockfile")
    }

    fn write_state(&self) -> Result {
        util::write_json(
            self.path().join(Self::PROFILE_STATE_FILE_NAME),
            self.state.data(),
        )
        .context("failed to write profile state")
    }

    pub fn path(&self) -> &Path {
        self.state.path()
    }

    pub fn game(&self) -> &str {
        &self.manifest.profile.game
    }

    pub async fn resolve(&self, ctx: &Context, update: bool) -> Result<Lockfile> {
        let span = tracing::info_span!("resolve_manifest");
        span.pb_set_style(&ProgressStyle::default_spinner());
        span.pb_set_message("resolving dependencies...");

        let _enter = span.enter();

        loadsmith::manifest::resolve(
            self.manifest.mods.clone().into_dependencies(),
            &ctx.registry_set,
            if update { None } else { Some(&self.lockfile) },
        )
        .await
        .context("error while resolving manifest")
    }

    pub async fn resolve_and_update_lockfile(&mut self, ctx: &Context, update: bool) -> Result {
        let new_lockfile = self.resolve(ctx, update).await?;

        let diff = self.lockfile.diff(&new_lockfile);
        crate::fmt::log_lockfile_diff(&diff);

        self.lockfile = new_lockfile;
        Ok(())
    }

    pub async fn sync(&mut self, ctx: Arc<Context>) -> Result {
        let diff = self.state.diff_lockfile(&self.lockfile);

        if diff.is_empty() {
            info!("profile is up to date");
            return Ok(());
        }

        let to_remove = diff.to_remove().cloned().collect::<Vec<_>>();
        let mut to_add = diff.to_add().cloned().collect::<Vec<_>>();

        if !to_remove.is_empty() {
            info!("{} packages to uninstall", to_remove.len());

            for package in to_remove {
                debug!("uninstalling {}", package.ref_.id);

                self.state.uninstall(&package.ref_.id)?;
            }
        }

        if !to_add.is_empty() {
            let span = tracing::info_span!("install_packages");
            span.pb_set_style(
                &ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>10}/{len:10}: {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            span.pb_set_message("installing packages...");
            span.pb_set_length(to_add.len() as u64);

            let _enter = span.enter();

            let schema = self
                .get_schema(&ctx)
                .await
                .map(Arc::new)
                .context("failed to fetch thunderstore schema")?;

            let loader = self
                .make_loader(&schema)
                .map(Arc::from)
                .context("failed to configure loader")?;

            let semaphore = Arc::new(tokio::sync::Semaphore::new(4));

            let (tx, mut rx) = mpsc::channel::<Result<PackageRef>>(to_add.len());

            to_add.sort_by_key(|pkg| pkg.size.unwrap_or(0));
            to_add.reverse();

            for package in to_add {
                let schema = Arc::clone(&schema);
                let loader = Arc::clone(&loader);
                let ctx = Arc::clone(&ctx);
                let tx = tx.clone();

                let semaphore = Arc::clone(&semaphore);
                let acquire = semaphore.acquire_owned();

                tokio::spawn(async move {
                    let _permit = acquire.await;

                    let ruleset = Self::get_ruleset_for(&schema, &*loader, &package.ref_);
                    let msg = match Self::download_and_install(ruleset, &package, &ctx).await {
                        Ok(_) => Ok(package.ref_),
                        Err(err) => Err(err),
                    };

                    drop(_permit);

                    if let Err(err) = tx.send(msg).await {
                        warn!("failed to send package install result: {err}");
                    }
                });
            }

            drop(tx);

            while let Some(res) = rx.recv().await {
                let ref_ = res.context("failed to install package")?;

                let ruleset = Self::get_ruleset_for(&schema, &*loader, &ref_);
                let store_dir = store_dir(&ref_, &ctx);
                self.state
                    .install(ref_, ruleset, &store_dir, false)
                    .context("failed to install package")?;

                self.write_state()?;

                span.pb_inc(1);
            }
        }

        Ok(())
    }

    async fn download_and_install(
        ruleset: InstallRuleset<'_>,
        package: &LockedPackage,
        ctx: &Context,
    ) -> Result<()> {
        debug!("installing {}", package.ref_.id);

        let package_span = tracing::info_span!("install_package");
        package_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(
                    "[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes:>10}/{total_bytes:10}: {msg}",
                )
                .unwrap()
                .progress_chars("=>-"),
        );

        package_span.pb_set_message(package.ref_.id.as_str());
        if let Some(size) = package.size {
            package_span.pb_set_length(size);
        }

        let _package_enter = package_span.enter();

        let store_dir = store_dir(&package.ref_, ctx);

        if !store_dir.exists() {
            let mut vec = Vec::with_capacity(package.size.unwrap_or(0) as usize);
            if let Some(path) = package.url.strip_prefix("file://") {
                let mut file = File::open(path)?;
                file.read_to_end(&mut vec)?;

                package_span.pb_inc(vec.len() as u64);
            } else {
                let mut stream = ctx.http.get(&package.url).send().await?.bytes_stream();

                while let Some(chunk) = stream.try_next().await? {
                    vec.extend_from_slice(&chunk);
                    package_span.pb_inc(chunk.len() as u64);
                }
            }

            fs::create_dir_all(&store_dir).context("failed to create package store directory")?;

            loadsmith::install::extract(Cursor::new(vec), &package.ref_, ruleset, &store_dir)
                .context("failed to extract package")?;
        }

        Ok(())
    }

    pub async fn get_schema(&self, ctx: &Context) -> Result<ThunderstoreSchema> {
        ctx.thunderstore.get_schema("dev").await.map_err(Into::into)
    }

    pub fn make_loader(&self, schema: &ThunderstoreSchema) -> Result<Box<dyn Loader>> {
        let game = schema.games.get(self.game()).context("unknown game")?;
        let config = game
            .r2modman
            .iter()
            .flatten()
            .next()
            .context("no r2modman config found for game")?;
        let loader = loadsmith::thunderstore::r2_config_to_loader(config)
            .context("failed to convert r2modman config to loader")?
            .context("unsupported game")?;

        Ok(loader)
    }

    pub fn get_ruleset_for<'a>(
        schema: &ThunderstoreSchema,
        loader: &'a dyn Loader,
        pkg: &PackageRef,
    ) -> InstallRuleset<'a> {
        let is_loader = schema
            .modloader_packages
            .iter()
            .any(|loader_pkg| loader_pkg.package_id.as_str() == pkg.id.as_str());

        if is_loader {
            loader.loader_install_rules()
        } else {
            loader.package_install_rules()
        }
    }
}

fn store_dir(package: &PackageRef, ctx: &Context) -> PathBuf {
    let prefix = package
        .id
        .as_str()
        .chars()
        .take(2)
        .collect::<String>()
        .to_lowercase();

    let store_dir = ctx
        .home_dir
        .join("store")
        .join(prefix)
        .join(package.id.as_str())
        .join(package.version.to_string());

    store_dir
}
