use anyhow::Context;
use dialoguer::{Select, theme::ColorfulTheme};
use nondestructive::yaml::StringKind;

use crate::{
    backends::factory::init_backend,
    cli::DeployCmd,
    config::{Config, Status, UpdateSet, read_config},
};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub fn run(cmd: DeployCmd) -> anyhow::Result<()> {
    let config = read_config(cmd.config)?;
    let mut backend = init_backend(config.backend.clone())?;
    let updateset = if let Some(updateset) = cmd.updateset {
        updateset
    } else {
        let updatesets = backend.list_updatesets()?;
        let updateset = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select updateset")
            .items(&updatesets)
            .interact()?;
        updatesets.get(updateset).unwrap().to_owned()
    };
    println!("Using updateset {updateset}");
    let mut updateset = backend.get_updateset(&updateset)?;

    // Get stage
    let stage = if let Some(stage) = cmd.stage {
        stage
    } else if let Some(stage) = next_stage(&config, &updateset) {
        stage
    } else {
        println!("No next stage found");
        return Ok(());
    };

    // get list of clusters based on stage
    let clusters = cluster_set(&config, &stage)?;

    let base_dir = config.base_dir.unwrap_or_else(|| PathBuf::from("."));

    for cluster in clusters {
        edit_cluster(&updateset, &cluster, &base_dir)?;
    }

    if cmd.record {
        let status = updateset.status.get_or_insert_with(|| Status {
            stages_completed: Vec::new(),
        });
        status.stages_completed.push(stage);
        backend.update_updateset(updateset.id.clone(), updateset)?;
    }

    Ok(())
}

fn next_stage(config: &Config, updateset: &UpdateSet) -> Option<String> {
    if let Some(status) = updateset.status.as_ref() {
        for stage in config.stages.iter() {
            if !status.stages_completed.contains(&stage.name) {
                return Some(stage.name.clone());
            }
        }
        None
    } else {
        config.stages.first().map(|s| s.name.clone())
    }
}

fn cluster_set(config: &Config, stage_name: &str) -> anyhow::Result<Vec<String>> {
    for stage in &config.stages {
        if stage.name == stage_name {
            return Ok(stage.clusters.clone());
        }
    }
    anyhow::bail!("No stage with name {} found", stage_name)
}

fn edit_cluster(updateset: &UpdateSet, cluster_name: &str, base_dir: &Path) -> anyhow::Result<()> {
    for change in updateset.updates.iter() {
        let path = base_dir
            .join("clusters")
            .join(cluster_name)
            .join(&change.file);
        let Ok(data) = std::fs::read_to_string(&path) else {
            println!(
                "No file {} found for cluster {cluster_name}. Ignoring",
                change.file
            );
            continue;
        };
        let mut document = nondestructive::yaml::from_slice(&data)?;
        let mut root = document
            .as_mut()
            .into_mapping_mut()
            .context("root mapping not found")?;
        let mut spec = root.get_mut("spec").context("spec not found")?;
        let mut spec = spec.as_mapping_mut().context("spec not a mapping")?;
        let mut post_build = spec.get_mut("postBuild").context("postBuild not found")?;
        let mut post_build = post_build
            .as_mapping_mut()
            .context("postBuild is not a mapping")?;
        let mut substitute = post_build
            .get_mut("substitute")
            .context("substitute not found")?;
        let mut substitute = substitute
            .as_mapping_mut()
            .context("substitute is not a mapping")?;

        if let Some(mut value) = substitute.get_mut(&change.post_build_variable) {
            value.set_string_with(&change.version, StringKind::Double);
        } else {
            substitute
                .insert(
                    &change.post_build_variable,
                    nondestructive::yaml::Separator::Auto,
                )
                .set_string_with(&change.version, StringKind::Double);
        }
        let mut file = File::create(&path)?;
        document.write_to(&mut file)?;
    }
    Ok(())
}
