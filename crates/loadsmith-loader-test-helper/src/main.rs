use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use serde::Deserialize;
use tokio::fs;
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
struct Packages(HashMap<String, Vec<String>>);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let out_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out"));

    let packages_json = include_str!("../packages.json");
    let packages: Packages = serde_json::from_str(packages_json)?;

    let http = reqwest::Client::new();

    for (category, package_list) in packages.0 {
        let category_path = out_path.join(&category);
        fs::create_dir_all(&category_path).await?;

        for package_id in package_list {
            let bytes = download_package(&http, &package_id)
                .await
                .with_context(|| format!("failed to download {package_id}"))?;

            debug!(
                category,
                package_id,
                len = bytes.len(),
                "downloaded package"
            );

            let file_list = extract_file_list(bytes)
                .await
                .with_context(|| format!("failed to extract file list for {package_id}"))?;
            let contents = file_list.join("\n");

            let package_path = category_path.join(format!("{package_id}.txt"));
            fs::write(&package_path, contents).await?;

            info!(
                category,
                package_id,
                file_count = file_list.len(),
                path = %package_path.display(),
                "wrote package list"
            );
        }
    }

    Ok(())
}

pub async fn download_package(http: &reqwest::Client, thunderstore_id: &str) -> Result<Bytes> {
    let mut split = thunderstore_id.split('-');
    let (Some(author), Some(name), Some(version)) = (split.next(), split.next(), split.next())
    else {
        bail!("invalid thunderstore package id: {thunderstore_id}")
    };

    let url = format!("https://thunderstore.io/package/download/{author}/{name}/{version}/",);

    debug!(thunderstore_id, %url, "downloading package");

    let response = http
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(response)
}

pub async fn extract_file_list(bytes: Bytes) -> Result<Vec<String>> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    let file_list = (0..archive.len())
        .filter_map(|i| match archive.by_index(i) {
            Ok(entry) if entry.is_dir() => None,
            Ok(entry) => Some(Ok(entry.mangled_name().to_string_lossy().into_owned())),
            Err(err) => Some(Err(anyhow!(err).context("failed to read zip entry"))),
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(file_list)
}
