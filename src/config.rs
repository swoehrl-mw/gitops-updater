use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub base_dir: Option<PathBuf>,
    pub releases: Vec<Release>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_clusters: Option<Vec<String>>,
    pub stages: Vec<Stage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Release {
    #[serde(flatten)]
    pub variant: ReleaseVariant,
    pub target: Target,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseVariant {
    Helm(Helm),
    Container(Container),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Helm {
    pub repo: String,
    pub chart: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub image: String,
    pub version_regex: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub file: String,
    pub post_build_variable: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub name: String,
    pub clusters: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum BackendConfig {
    Local { path: Option<PathBuf> },
    Github { repo: String },
}

pub fn read_config(path: Option<PathBuf>) -> anyhow::Result<Config> {
    let config_path = path.unwrap_or_else(|| PathBuf::from("updater.yaml"));
    let config_content = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
    let config: Config = serde_yaml::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
    Ok(config)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSet {
    pub id: String,
    pub updates: Vec<Update>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub file: String,
    pub post_build_variable: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub stages_completed: Vec<String>,
}
