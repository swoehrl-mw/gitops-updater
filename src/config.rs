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
    pub name: Option<String>,
    #[serde(flatten)]
    pub variant: ReleaseVariant,
    pub target: Target,
}

impl Release {
    pub fn name(&self) -> String {
        if let Some(name) = self.name.as_ref() {
            name.clone()
        } else {
            self.variant.name()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseVariant {
    Helm(Helm),
    Container(Container),
}

impl ReleaseVariant {
    pub fn name(&self) -> String {
        match self {
            ReleaseVariant::Helm(helm) => helm.name(),
            ReleaseVariant::Container(container) => container.name(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Helm {
    pub repo: String,
    pub chart: String,
    pub version_regex: Option<String>,
}

impl Helm {
    pub fn name(&self) -> String {
        self.chart.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub image: String,
    pub version_regex: Option<String>,
}

impl Container {
    pub fn name(&self) -> String {
        self.image.clone()
    }
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
    #[serde(skip_serializing, default)]
    pub name: String,
    pub file: String,
    pub post_build_variable: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub stages_completed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_target() -> Target {
        Target {
            file: "some-file".to_owned(),
            post_build_variable: "someVersion".to_owned(),
        }
    }

    #[test]
    fn release_name_prefers_explicit_name() {
        let release = Release {
            name: Some("explicit-name".to_owned()),
            variant: ReleaseVariant::Helm(Helm {
                repo: "some-repo".to_owned(),
                chart: "chart-name".to_owned(),
                version_regex: None,
            }),
            target: dummy_target(),
        };

        assert_eq!(release.name(), "explicit-name".to_owned());
    }

    #[test]
    fn release_name_falls_back_to_helm_chart() {
        let release = Release {
            name: None,
            variant: ReleaseVariant::Helm(Helm {
                repo: "some-repo".to_owned(),
                chart: "chart-name".to_owned(),
                version_regex: None,
            }),
            target: dummy_target(),
        };

        assert_eq!(release.name(), "chart-name".to_owned());
    }

    #[test]
    fn release_name_falls_back_to_container_image() {
        let release = Release {
            name: None,
            variant: ReleaseVariant::Container(Container {
                image: "some-image".to_owned(),
                version_regex: None,
            }),
            target: dummy_target(),
        };

        assert_eq!(release.name(), "some-image".to_owned());
    }
}
