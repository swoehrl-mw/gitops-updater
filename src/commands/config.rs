use anyhow::Context;

use crate::{
    cli::GenerateConfigCmd,
    config::{Config, Helm, Release, ReleaseVariant, Stage, Target},
    schemas::{
        helmrelease::HelmRelease, helmrepository::HelmRepository, kustomization::Kustomization,
        ocirepository::OciRepository,
    },
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub fn run(cmd: GenerateConfigCmd) -> anyhow::Result<()> {
    // go through apps folder
    // for each folder, check all files, find HelmRelease and HelmRepository
    let mut root = cmd.root.clone().unwrap_or_default();

    let config_basedir = if &root == "." {
        root = "".into();
        None
    } else {
        Some(root.clone())
    };
    let mut repos = HashMap::new();
    let mut helm_releases = Vec::new();
    let apps_path = root.join("apps");
    if apps_path.is_dir() {
        for entry in fs::read_dir(&apps_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_helm(&path, &mut repos, &mut helm_releases)?;
            }
        }
    }

    let mut clusters = Vec::new();
    let clusters_path = root.join("clusters");
    for entry in fs::read_dir(&clusters_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            clusters.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    clusters.sort();
    let stages = vec![Stage {
        name: "all".to_owned(),
        clusters,
    }];

    let mut releases = Vec::new();

    for helm in helm_releases {
        let filename = find_file_for_app_path(&root, &helm.app_folder);
        let Ok(Some(file)) = filename else {
            println!("Could not find filename for app {:?}", helm.app_folder);
            continue;
        };

        let repo_url = if let Some(name) = helm.repo_name {
            let namespace = helm.repo_namespace.clone().unwrap();
            let Some(repo_url) = repos.get(&(name.clone(), namespace.clone())) else {
                println!("No repo url for {namespace}/{name}");
                continue;
            };
            repo_url.to_owned()
        } else if let Some(repo_url) = helm.repo_url {
            repo_url
        } else {
            anyhow::bail!("Neither repo_url nor repo_name found in {file}");
        };

        releases.push(Release {
            name: None,
            variant: ReleaseVariant::Helm(Helm {
                repo: repo_url,
                chart: helm.chart_name,
                version_regex: None,
            }),
            target: Target {
                file,
                post_build_variable: helm.variable,
            },
        });
    }

    // Make sure we don't have duplicates due to version variables used multiple times
    releases.sort();
    releases.dedup();

    let config = Config {
        base_dir: config_basedir,
        releases,
        stages,
        reference_clusters: None,
        backend: None,
    };

    let config = serde_yaml::to_string(&config)?;
    let config_path = cmd.config.unwrap_or_else(|| "updater.yaml".into());

    std::fs::write(&config_path, &config)?;
    Ok(())
}

#[derive(Default, Debug)]
struct HelmInfo {
    pub repo_name: Option<String>,
    pub repo_namespace: Option<String>,
    pub repo_url: Option<String>,
    pub chart_name: String,
    pub variable: String,
    pub app_folder: PathBuf,
}

fn collect_helm(
    dir: &Path,
    repos: &mut HashMap<(String, String), String>,
    helm_releases: &mut Vec<HelmInfo>,
) -> anyhow::Result<()> {
    collect_helm_inner(dir, dir, repos, helm_releases)?;
    Ok(())
}

fn collect_helm_inner(
    app_dir: &Path,
    dir: &Path,
    repos: &mut HashMap<(String, String), String>,
    helm_releases: &mut Vec<HelmInfo>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_helm_inner(app_dir, &entry.path(), repos, helm_releases)?;
        } else if entry_path.is_file() {
            find_helm_in_file(app_dir, &entry_path, repos, helm_releases)?;
        }
    }
    Ok(())
}

fn find_helm_in_file(
    app_dir: &Path,
    path: &Path,
    repos: &mut HashMap<(String, String), String>,
    helm_releases: &mut Vec<HelmInfo>,
) -> anyhow::Result<()> {
    let data = std::fs::read_to_string(path)?;
    for part in data.split("\n---\n") {
        let part = part.trim_end_matches("---");
        let data = match serde_yaml::from_str::<serde_yaml::Value>(part) {
            Ok(data) => data,
            Err(err) => {
                println!("Warning: Can not parse {path:?} as yaml: {err}");
                return Ok(());
            }
        };

        let Some(kind) = data.get("kind").and_then(|v| v.as_str()) else {
            return Ok(());
        };

        if kind == "HelmRelease" {
            let release: HelmRelease = serde_yaml::from_value(data)
                .context(format!("could not parse HelmRelease in {path:?}"))?;
            let Some(chart) = release.spec.chart else {
                continue;
            };
            let chart_spec = chart.spec;

            let Some(chart_version) = chart_spec.version else {
                println!("HelmRelease in {path:?} has no version. Ignoring");
                return Ok(());
            };
            if !chart_version.starts_with("${") {
                println!(
                    "WARN: version for HelmRelease {} is not a variable",
                    release.k8s_base.metadata.name
                );
                return Ok(());
            }

            let variable = variable_name(&chart_version);

            let repo_namespace = Some(
                release
                    .k8s_base
                    .metadata
                    .namespace
                    .clone()
                    .unwrap_or_else(|| "default".to_owned()),
            );

            helm_releases.push(HelmInfo {
                repo_name: Some(chart_spec.source_ref.name.clone()),
                repo_namespace,
                repo_url: None,
                app_folder: app_dir.to_path_buf(),
                chart_name: chart_spec.chart.clone(),
                variable,
            });
        } else if kind == "HelmRepository" {
            let repo: HelmRepository = serde_yaml::from_value(data)
                .context(format!("could not parse HelmRepository in {path:?}"))?;
            let meta = repo.k8s_base.metadata;

            repos.insert(
                (
                    meta.name.clone(),
                    meta.namespace
                        .clone()
                        .unwrap_or_else(|| "default".to_owned()),
                ),
                repo.spec.url,
            );
        } else if kind == "OCIRepository" {
            let repo: OciRepository = serde_yaml::from_value(data)
                .context(format!("could not parse OCIRepository in {path:?}"))?;

            let (repo_url, chart_name) = repo.spec.url.rsplit_once('/').unwrap();
            helm_releases.push(HelmInfo {
                repo_url: Some(repo_url.to_owned()),
                chart_name: chart_name.to_owned(),
                variable: variable_name(&repo.spec.reference.tag),
                app_folder: app_dir.to_path_buf(),
                ..Default::default()
            });
        }
    }

    Ok(())
}

fn variable_name(chart_version: &str) -> String {
    if chart_version.contains(":") {
        chart_version
            .replace("${", "")
            .split_once(":")
            .unwrap()
            .0
            .to_owned()
    } else {
        chart_version
            .replace("${", "")
            .replace("}", "")
            .trim()
            .to_owned()
    }
}

fn find_file_for_app_path(base_path: &Path, app_path: &Path) -> anyhow::Result<Option<String>> {
    let dir = base_path.join("clusters");
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            for entry in fs::read_dir(entry_path)? {
                let entry = entry?;
                let entry_path = entry.path();
                if !entry_path.is_file() {
                    continue;
                }
                if app_path_in_file(&entry_path, app_path).unwrap_or(false) {
                    return Ok(Some(
                        entry_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                    ));
                }
            }
        }
    }
    Ok(None)
}

fn app_path_in_file(file_path: &Path, app_path: &Path) -> anyhow::Result<bool> {
    let data = std::fs::read_to_string(file_path)?;
    if let Ok(kustomization) = serde_yaml::from_str::<Kustomization>(&data) {
        let path = kustomization.spec.path;
        let path = path.trim_start_matches("./").trim_matches('/');
        return Ok(Path::new(path) == app_path);
    }
    Ok(false)
}
