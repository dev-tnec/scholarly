use clap::{Parser, Subcommand};
use scholarly::{config::Config, import, routes};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "scholarly", version, about = "Scholarly scholarship directory")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server
    Serve,
    /// Import the master workbook into Postgres
    Import {
        /// Path to the .xlsx workbook
        #[arg(long)]
        file: std::path::PathBuf,
        /// Validate only, write nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Re-run import validation against the live database
    Check,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "scholarly=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::from_env()?;

    match cli.command {
        Command::Serve => {
            let state = routes::AppState::connect(config).await?;
            routes::serve(state).await?;
        }
        Command::Import { file, dry_run } => {
            let state = routes::AppState::connect(config).await?;
            import::run(&state, &file, dry_run).await?;
        }
        Command::Check => {
            let state = routes::AppState::connect(config).await?;
            let report = import::check(&state).await?;
            report.print();
            if !report.ok() {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
