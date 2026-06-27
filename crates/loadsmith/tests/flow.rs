use std::io::Cursor;

use anyhow::{Context, anyhow};
use loadsmith_core::PackageRef;
use loadsmith_thunderstore::PackageRefExt;
use walkdir::WalkDir;

#[tokio::test]
#[ignore]
async fn install_flow() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = thunderstore::Client::new();

    let schema = client.get_schema("dev").await?;
    let lethal_company = schema
        .games
        .get("lethal-company")
        .ok_or_else(|| anyhow!("lethal-company not found"))?;
    let r2_config = lethal_company
        .r2modman
        .as_ref()
        .and_then(|vec| vec.first())
        .ok_or_else(|| anyhow!("r2modman config not found"))?;
    let loader = loadsmith::thunderstore::r2_config_to_loader(r2_config)?
        .ok_or_else(|| anyhow!("loader couldn't be created"))?;

    let package = PackageRef::new("Evaisa-LethalLib".to_string(), (1, 2, 0));
    let bytes = client.download(package.clone().into_ts_ident()?).await?;

    let extract_dir = tempfile::tempdir()?;
    loadsmith::install::extract_from_reader(
        Cursor::new(bytes),
        &package,
        loader.package_install_rules(),
        extract_dir.path(),
    )?;

    let install_dir = tempfile::tempdir()?;
    let (install_manifest, overwritten_files) = loadsmith::install::install(
        package.clone(),
        extract_dir.path(),
        install_dir.path(),
        loader.package_install_rules(),
        false,
    )
    .context("failed to install")?;

    assert!(overwritten_files.is_empty());
    assert_eq!(install_manifest.package, package);

    let mut files = WalkDir::new(install_dir.path())
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            if entry.file_type().is_dir() {
                return None;
            }

            let relative_path = entry
                .path()
                .strip_prefix(install_dir.path())
                .expect("entry path should be relative to install dir");

            let relative_path = relative_path.to_string_lossy().to_string();
            Some(relative_path)
        })
        .collect::<Vec<_>>();

    files.sort();

    insta::assert_yaml_snapshot!(files);

    Ok(())
}
