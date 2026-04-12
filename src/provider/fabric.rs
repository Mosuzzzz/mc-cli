use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct FabricGameVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct SubVersion {
    version: String,
    stable: bool,
}

pub async fn list_versions() -> Result<Vec<String>> {
    let url = "https://meta.fabricmc.net/v2/versions/game";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.error_for_status()?;
    let versions: Vec<FabricGameVersion> = response
        .json()
        .await
        .context("Failed to parse Fabric API response")?;

    // API returns newest-first; keep only stable versions
    let stables = versions
        .into_iter()
        .filter(|v| v.stable)
        .map(|v| v.version)
        .collect();

    Ok(stables)
}

pub async fn download(version: &str, dest: &std::path::Path) -> Result<()> {
    let client = reqwest::Client::new();

    // 1. Get latest stable loader
    let loader_url = "https://meta.fabricmc.net/v2/versions/loader";
    let loaders: Vec<SubVersion> = client
        .get(loader_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let latest_loader = loaders
        .into_iter()
        .find(|l| l.stable)
        .context("No stable loader found")?;

    // 2. Get latest stable installer
    let installer_url = "https://meta.fabricmc.net/v2/versions/installer";
    let installers: Vec<SubVersion> = client
        .get(installer_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let latest_installer = installers
        .into_iter()
        .find(|i| i.stable)
        .context("No stable installer found")?;

    // 3. Download the server jar
    let download_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
        version, latest_loader.version, latest_installer.version
    );

    let response = client.get(&download_url).send().await?.error_for_status()?;

    let mut file = std::fs::File::create(dest)?;
    let bytes = response.bytes().await?;

    use std::io::Write;
    file.write_all(&bytes)?;

    Ok(())
}
