pub mod fabric;
pub mod paper;
pub mod vanilla;

use anyhow::Result;

pub enum GameProvider {
    Paper,
    Vanilla,
    Fabric,
}

impl GameProvider {
    pub fn from_str(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "paper" => Some(Self::Paper),
            "vanilla" => Some(Self::Vanilla),
            "fabric" => Some(Self::Fabric),
            _ => None,
        }
    }

    pub async fn list_versions(&self) -> Result<Vec<String>> {
        match self {
            Self::Paper => paper::list_versions().await,
            Self::Vanilla => vanilla::list_versions().await,
            Self::Fabric => fabric::list_versions().await,
        }
    }

    pub async fn download(&self, version: &str, dest: &std::path::Path) -> Result<()> {
        match self {
            Self::Paper => paper::download(version, dest).await,
            Self::Vanilla => vanilla::download(version, dest).await,
            Self::Fabric => fabric::download(version, dest).await,
        }
    }
}
