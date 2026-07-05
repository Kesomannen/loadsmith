use std::{
    fs::File,
    io::{Cursor, Read},
};

use anyhow::Context as _;
use futures::TryStreamExt;
use loadsmith::{
    core::PackageRef,
    install::InstallRuleset,
    loader::Loader,
    manifest::{LockedPackage, Lockfile, ProfileState},
};
use thunderstore::models::schema::Schema as ThunderstoreSchema;
use tracing::{debug, info};
use tracing_indicatif::{span_ext::IndicatifSpanExt, style::ProgressStyle};

use crate::{Context, Result, manifest::Manifest};

#[derive(Debug)]
pub struct Profile {
    pub manifest: Manifest,
    pub lockfile: Lockfile,
    pub state: ProfileState,
}

impl Profile {
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

    pub async fn sync(&mut self, ctx: &Context) -> Result {
        let diff = self.state.diff_lockfile(&self.lockfile);

        if diff.is_empty() {
            info!("profile is up to date");
            return Ok(());
        }

        let to_remove = diff.to_remove().cloned().collect::<Vec<_>>();
        let to_add = diff.to_add().cloned().collect::<Vec<_>>();

        if !to_remove.is_empty() {
            info!("{} packages to uninstall", to_remove.len());

            for package in to_remove {
                debug!("uninstalling {}", package.ref_.id);

                self.state.uninstall(&package.ref_.id)?;
            }
        }

        let schema = self
            .get_schema(ctx)
            .await
            .context("failed to fetch thunderstore schema")?;

        let loader = self
            .make_loader(&schema)
            .context("failed to configure loader")?;

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

            for package in to_add {
                let ruleset = self.get_ruleset_for(&schema, &*loader, &package.ref_);

                self.download_and_install(ruleset, package, ctx).await?;

                span.pb_inc(1);
            }
        }

        Ok(())
    }

    async fn download_and_install(
        &mut self,
        ruleset: InstallRuleset<'_>,
        package: LockedPackage,
        ctx: &Context,
    ) -> Result {
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

        let extract_dir = tempfile::tempdir()?;
        loadsmith::install::extract(Cursor::new(vec), &package.ref_, ruleset, &extract_dir)
            .context("failed to extract package")?;

        self.state
            .install(package.ref_, ruleset, &extract_dir, true)
            .context("failed to install package")?;

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
        &self,
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
