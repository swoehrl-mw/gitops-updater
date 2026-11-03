use std::path::Path;

use rand::distr::{Alphanumeric, SampleString};

use crate::{
    backends::{factory::init_backend, interface::StorageBackend},
    cli::{UpdatesetCmd, UpdatesetSubCmd},
    config::{Config, Helm, ReleaseVariant, Update, UpdateSet, read_config},
    schemas::kustomization::Kustomization,
    sources::{helm, oci},
};

pub async fn run(cmd: UpdatesetCmd) -> anyhow::Result<()> {
    let config = read_config(cmd.config)?;
    let mut backend = init_backend(config.backend.clone())?;
    let result = match cmd.subcmd {
        Some(UpdatesetSubCmd::New) | None => {
            generate_new_updateset(config, backend.as_mut()).await?
        }
    };
    if cmd.exit_with_result && result == CommandResult::NoChange {
        std::process::exit(2);
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
enum CommandResult {
    NoChange,
    NewUpdateset,
}

async fn generate_new_updateset(
    config: Config,
    backend: &mut dyn StorageBackend,
) -> anyhow::Result<CommandResult> {
    let mut changes = Vec::new();
    // find newest versions
    for release in config.releases.clone() {
        let name = release.name();
        let version = match release.variant {
            ReleaseVariant::Helm(helm) => helm_version(helm).await?,
            ReleaseVariant::Container(container) => {
                oci::find_newest_version_of_container_image(
                    &container.image,
                    &container.version_regex,
                )
                .await?
            }
        };
        if let Some(version) = version {
            changes.push(Update {
                name,
                file: release.target.file.clone(),
                post_build_variable: release.target.post_build_variable,
                version,
            });
        }
    }
    let changes = check_for_actual_updates(config, changes)?;
    if changes.is_empty() {
        println!("No updates found, not generating an updateset");
        return Ok(CommandResult::NoChange);
    }

    let mut names: Vec<String> = changes.iter().map(|u| u.name.clone()).collect();
    names.sort();

    let set = UpdateSet {
        id: generate_id(),
        updates: changes,
        status: None,
    };
    backend.update_updateset(set.id.clone(), set)?;

    println!("Found updates for: {}", names.join(", "));

    Ok(CommandResult::NewUpdateset)
}

async fn helm_version(helm: Helm) -> anyhow::Result<Option<String>> {
    if helm.repo.starts_with("http") {
        helm::find_newest_version_of_chart(&helm.repo, &helm.chart, &helm.version_regex).await
    } else if helm.repo.starts_with("oci://") {
        oci::find_newest_version_of_chart(
            &helm.repo.replace("oci://", ""),
            &helm.chart,
            &helm.version_regex,
        )
        .await
    } else {
        anyhow::bail!("Unknown protocol: {}", helm.repo);
    }
}

fn check_for_actual_updates(config: Config, changes: Vec<Update>) -> anyhow::Result<Vec<Update>> {
    let base_path = config.base_dir.unwrap_or_default();
    let clusters = if let Some(reference_clusters) = config.reference_clusters {
        reference_clusters
    } else {
        let mut clusters = Vec::new();
        for stage in config.stages.iter() {
            clusters.append(&mut stage.clusters.clone());
        }
        clusters
    };
    let mut result = Vec::new();
    for change in changes {
        for cluster in clusters.iter() {
            if let Some(version) = get_version_from_kustomization(
                &base_path,
                &change.file,
                cluster,
                &change.post_build_variable,
            )? && version != change.version
            {
                result.push(change);
                break;
            }
        }
    }
    Ok(result)
}

fn get_version_from_kustomization(
    base_path: &Path,
    filename: &str,
    cluster_name: &str,
    post_build_variable: &str,
) -> anyhow::Result<Option<String>> {
    let path = base_path.join("clusters").join(cluster_name).join(filename);
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let kustomization = serde_saphyr::from_str::<Kustomization>(&data)?;
    let res = kustomization
        .spec
        .post_build
        .as_ref()
        .and_then(|p| p.substitute.as_ref())
        .and_then(|s| s.get(post_build_variable).to_owned())
        .cloned();
    Ok(res)
}

const ID_LENGTH: usize = 8;

fn generate_id() -> String {
    let mut rng = rand::rng();
    Alphanumeric.sample_string(&mut rng, ID_LENGTH)
}
