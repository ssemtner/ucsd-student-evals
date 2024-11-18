use diesel::{BelongingToDsl, ExpressionMethods};
mod common;
mod cookies;
mod courses;
mod database;
mod evaluations;
mod schema;

use crate::courses::get_all_courses;
use crate::database::{establish_connection, Course, Evaluation, SectionId};
use crate::evaluations::save_eval;
use crate::evaluations::sids::save_all_sids;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use diesel::{QueryDsl, RunQueryDsl};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::OnceCell;

static SETTINGS: OnceCell<Settings> = OnceCell::const_new();

pub(crate) fn settings() -> &'static Settings {
    SETTINGS.get().unwrap()
}

#[derive(Deserialize, Debug)]
struct Settings {
    proxy_url: String,
    proxy_username: String,
    proxy_password: String,
    cookies_token: String,
    database_url: String,
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
    Sids,
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
        } => {
            courses::display_stats(&mut conn)?;
        }
        Commands::Evals {
            command: EvalCommands::Sids,
        } => {
            save_all_sids(&mut conn).await?;
        }
        Commands::Evals {
            command: EvalCommands::Fetch,
        } => {
            let course = schema::courses::table
                .filter(schema::courses::code.eq("CSE|123"))
                .get_result::<Course>(&mut conn)?;
            let sids = SectionId::belonging_to(&course)
                .select(schema::sids::sid)
                .load::<i32>(&mut conn)?;
            println!("{:?}", course);
            println!("{:?} = {}", sids, sids.len());
            let client = common::client()?;
            for sid in sids {
                save_eval(&mut conn, &client, sid, &course).await?;
            }
        }
        Commands::Evals {
            command: EvalCommands::Stats,
        } => {
            let evals = schema::evaluations::table
                .count()
                .get_result::<i64>(&mut conn)?;
            let sections = schema::sids::table
                .filter(
                    schema::sids::sid
                        .ne_all(schema::evaluations::table.select(schema::evaluations::sid)),
                )
                .count()
                .get_result::<i64>(&mut conn)?;
            println!("{} evals", evals);
            println!("{} sections with no eval", sections);
        }
    }

    Ok(())
}
