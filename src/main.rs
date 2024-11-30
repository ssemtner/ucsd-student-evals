mod api;
mod common;
mod cookies;
mod courses;
mod database;
mod evaluations;
mod schema;

use crate::common::progress_bar;
use crate::courses::get_all_courses;
use crate::database::{establish_connection, Course, SectionId};
use crate::evaluations::save_evals;
use crate::evaluations::sids::save_all_sids;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use diesel::{BelongingToDsl, ExpressionMethods};
use diesel::{QueryDsl, RunQueryDsl};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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
    Serve {
        host: Option<String>,
    },
}

#[derive(Subcommand)]
enum CourseCommands {
    Stats,
    Fetch,
}

#[derive(Subcommand)]
enum EvalCommands {
    Stats,
    Fetch { course: Option<String> },
    Sids,
}

async fn reauth() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(ProgressStyle::with_template("{spinner:.blue} {msg}")?);
    pb.set_message("Fetching new cookies");

    cookies::fetch_cookies(&settings().cookies_token).await?;

    pb.finish_with_message("Done");

    Ok(())
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

    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Reauth => {
            reauth().await?;
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
            command: EvalCommands::Fetch { course: None },
        } => {
            let unprocessed_query = schema::sids::table
                .select(schema::sids::course_code)
                .filter(
                    schema::sids::sid
                        .ne_all(schema::evaluations::table.select(schema::evaluations::sid)),
                );

            let courses = schema::courses::table
                .filter(schema::courses::code.eq_any(unprocessed_query))
                .get_results::<Course>(&mut conn)?;
            let m = MultiProgress::new();

            let overall = m.add(common::progress_bar(courses.len() as u64));
            overall.set_message("Courses");

            for course in courses {
                let sids = SectionId::belonging_to(&course)
                    .select(schema::sids::sid)
                    .filter(
                        schema::sids::sid
                            .ne_all(schema::evaluations::table.select(schema::evaluations::sid)),
                    )
                    .get_results(&mut conn)?;
                let pb = m.insert_before(&overall, common::progress_bar(sids.len() as u64));
                pb.println(format!("Found {} sids for {}", sids.len(), course.code));
                let success = save_evals(&mut conn, &course, sids, &pb).await?;
                if !success {
                    reauth().await?;
                }
                pb.finish();
                overall.inc(1);
            }
            overall.finish();
            println!("Done");
        }
        Commands::Evals {
            command: EvalCommands::Fetch {
                course: Some(course),
            },
        } => {
            let course = schema::courses::table
                .filter(schema::courses::code.eq(course.replace(" ", "|")))
                .get_result::<Course>(&mut conn)?;
            let sids = SectionId::belonging_to(&course)
                .filter(
                    schema::sids::sid
                        .ne_all(schema::evaluations::table.select(schema::evaluations::sid)),
                )
                .select(schema::sids::sid)
                .get_results::<i32>(&mut conn)?;
            let pb = progress_bar(sids.len() as u64);
            save_evals(&mut conn, &course, sids, &pb).await?;
            pb.finish();
            println!("Done");
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
        Commands::Serve { host } => {
            let app = api::app()?;
            let listener =
                tokio::net::TcpListener::bind(host.unwrap_or("0.0.0.0:3000".to_string())).await?;
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}
