use std::collections::HashMap;

use regex::Regex;
use reqwest::{Client, header::AUTHORIZATION};
use semver::Version;
use serde::Deserialize;

use crate::sources::auth::auth_from_docker_config;

#[derive(Debug, Deserialize)]
pub struct HelmIndex {
    pub entries: HashMap<String, Vec<HelmChartEntry>>,
}

#[derive(Debug, Deserialize)]
pub struct HelmChartEntry {
    pub version: Option<String>,
}

pub async fn find_newest_version_of_chart(
    repo_url: &str,
    chart_name: &str,
    version_regex: &Option<String>,
) -> anyhow::Result<Option<String>> {
    let index = fetch_and_parse_helm_index(repo_url).await?;
    let version_regex = if let Some(regex) = version_regex {
        Some(Regex::new(regex)?)
    } else {
        None
    };
    Ok(newest_version(&index, chart_name, version_regex).and_then(|entry| entry.version.clone()))
}

pub async fn fetch_and_parse_helm_index(repo_url: &str) -> anyhow::Result<HelmIndex> {
    let index_url = format!("{}/index.yaml", repo_url.trim_end_matches('/'));
    let auth = auth_from_docker_config().unwrap_or_default();
    let client = Client::new();
    let host = host_from_url(repo_url);
    let mut request = client.get(&index_url);
    if let Some(credentials) = auth.get(&host) {
        request = request.header(AUTHORIZATION, format!("Basic {credentials}"));
    }
    let response = request.send().await?.text().await?;
    let index: HelmIndex = serde_saphyr::from_str(&response)?;
    Ok(index)
}

pub fn newest_version<'a>(
    index: &'a HelmIndex,
    chart_name: &str,
    version_regex: Option<Regex>,
) -> Option<&'a HelmChartEntry> {
    index
        .entries
        .get(chart_name)?
        .iter()
        .filter(|v| {
            if let Some(regex) = version_regex.as_ref()
                && let Some(version) = v.version.as_ref()
            {
                regex.is_match(version)
            } else {
                true
            }
        })
        .filter_map(|entry| {
            entry.version.as_ref().and_then(|v| {
                Version::parse(strip_version_prefix(v))
                    .ok()
                    .map(|ver| (ver, entry))
            })
        })
        .filter(|(v, _)| v.pre.is_empty())
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, entry)| entry)
}

fn strip_version_prefix(s: &str) -> &str {
    if s.starts_with('v') {
        s.split_at(1).1
    } else {
        s
    }
}

fn host_from_url(url: &str) -> String {
    let url = if let Some((_, url)) = url.split_once("://") {
        url
    } else {
        url
    };
    url.split_once('/')
        .map(|s| s.0)
        .unwrap_or_else(|| url)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_newest_version() {
        let yaml = r#"
entries:
  nginx:
    - version: "1.0.0"
    - version: "1.2.0"
    - version: "1.1.5"
    - version: "1.3.0-rc.1"
  redis:
    - version: "6.0.0"
    - version: "6.2.1"
"#;
        let index: HelmIndex = serde_saphyr::from_str(yaml).expect("Failed to parse HelmIndex");
        let regex = Regex::new(r"^\d+.\d+.\d+$").unwrap();
        let newest = newest_version(&index, "nginx", Some(regex)).expect("No version found");
        assert_eq!(newest.version.as_deref(), Some("1.2.0"));
        let newest_redis = newest_version(&index, "redis", None).expect("No version found");
        assert_eq!(newest_redis.version.as_deref(), Some("6.2.1"));
    }

    #[test]
    fn test_host_from_url() {
        assert_eq!(
            host_from_url("https://some.helm.repo"),
            "some.helm.repo".to_owned()
        );
        assert_eq!(
            host_from_url("https://some.helm.repo/charts"),
            "some.helm.repo".to_owned()
        );
        assert_eq!(
            host_from_url("https://some.helm.repo/charts/subpath"),
            "some.helm.repo".to_owned()
        );
        assert_eq!(
            host_from_url("some.helm.repo/charts"),
            "some.helm.repo".to_owned()
        );
    }
}
