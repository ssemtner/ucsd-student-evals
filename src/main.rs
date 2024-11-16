mod cookies;
mod courses;
mod database;
mod schema;

use crate::courses::get_all_courses;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::OnceCell;
use crate::database::establish_connection;

static SETTINGS: OnceCell<Settings> = OnceCell::const_new();

pub(crate) fn settings() -> &'static Settings {
    SETTINGS.get().unwrap()
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Courses {
        #[command(subcommand)]
        command: CourseCommands,
    },
    Evals {
        #[command(subcommand)]
        command: EvalCommands,
    },
    Reauth,
}

#[derive(Subcommand)]
enum CourseCommands {
    Stats,
    Fetch,
}

#[derive(Subcommand)]
enum EvalCommands {
    Stats,
    Fetch,
}

#[derive(Deserialize, Debug)]
struct Settings {
    base_url: String,
    cookies_token: String,
    database_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    {
        let settings = Config::builder()
            .add_source(config::File::with_name("config.toml"))
            .build()?;
        SETTINGS.set(settings.try_deserialize::<Settings>()?)?;
    }
    
    let mut conn = establish_connection()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Reauth => {
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(80));
            pb.set_style(ProgressStyle::with_template("{spinner:.blue} {msg}")?);
            pb.set_message("Fetching new cookies");

            cookies::fetch_cookies(&settings().cookies_token).await?;

            pb.finish_with_message("Done");
        }
        Commands::Courses {
            command: CourseCommands::Fetch,
        } => {
            get_all_courses(&mut conn).await?;
        }
        Commands::Courses {
            command: CourseCommands::Stats,
        } => {}
        Commands::Evals {
            command: EvalCommands::Fetch,
        } => {}
        Commands::Evals {
            command: EvalCommands::Stats,
        } => {}
    }

    Ok(())
}
