mod backends;
mod cli;
mod commands;
mod config;
mod schemas;
mod sources;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::cli();

    match cli.command {
        cli::Commands::Updateset(cmd) => {
            commands::updateset::run(cmd).await?;
        }
        cli::Commands::Deploy(cmd) => {
            commands::deploy::run(cmd)?;
        }
        cli::Commands::GenerateConfig(cmd) => {
            commands::config::run(cmd)?;
        }
    }
    Ok(())
}
