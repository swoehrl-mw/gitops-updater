use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitops-updater", about = "A GitOps update CLI tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a new config
    GenerateConfig(GenerateConfigCmd),
    /// Generate and manage updateset
    Updateset(UpdatesetCmd),
    /// Deploy the changes
    Deploy(DeployCmd),
}

#[derive(Args)]
pub struct UpdatesetCmd {
    #[arg(short = 'c', long = "config")]
    /// path to the configfile
    pub config: Option<PathBuf>,
    #[arg(short = 'e', long = "exit-code")]
    /// exit with code based on result (0=updateset generated, 2=no updates found, other=error)
    pub exit_with_result: bool,
    #[command(subcommand)]
    pub subcmd: Option<UpdatesetSubCmd>,
}

#[derive(Subcommand)]
pub enum UpdatesetSubCmd {
    /// Create a new updateset
    New,
}

#[derive(Args)]
pub struct DeployCmd {
    #[arg(short = 'c', long = "config")]
    /// path to the configfile
    pub config: Option<PathBuf>,
    #[arg(short = 'u', long = "updateset")]
    /// id of the updateset, if not given an interactive selection dialog will pop up
    pub updateset: Option<String>,
    #[arg(short = 's', long = "stage")]
    /// stage to deploy to next, if not given next available stage is used
    pub stage: Option<String>,
    #[arg(short = 'r', long = "record")]
    /// record deployed stage in updateset status
    pub record: bool,
}

#[derive(Args)]
pub struct GenerateConfigCmd {
    #[arg(short = 'c', long = "config")]
    pub config: Option<PathBuf>,
    pub root: Option<PathBuf>,
}

pub fn cli() -> Cli {
    Cli::parse()
}
