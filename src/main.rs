mod api;
mod common;
mod cookies;
mod courses;
mod database;
mod evaluations;

use crate::common::progress_bar;
use crate::courses::get_all_courses;
use crate::database::establish_connection;
use crate::evaluations::save_evals;
use crate::evaluations::sids::save_all_sids;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use courses::display_stats;
use database::Course;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Deserialize;
use sqlx::{query, query_as};
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

    let conn = establish_connection().await?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Reauth => {
            reauth().await?;
        }
        Commands::Courses {
            command: CourseCommands::Fetch,
        } => {
            get_all_courses(&conn).await?;
        }
        Commands::Courses {
            command: CourseCommands::Stats,
        } => {
            display_stats(&conn).await?;
        }
        Commands::Evals {
            command: EvalCommands::Sids,
        } => {
            save_all_sids(&conn).await?;
        }
        Commands::Evals {
            command: EvalCommands::Fetch { course: None },
        } => {
            let courses = query_as!(Course, "SELECT code, name, unit_id FROM courses WHERE code IN (SELECT course_code FROM sids WHERE sid NOT IN (SELECT sid FROM evaluations))")
                .fetch_all(&conn)
                .await?;

            let m = MultiProgress::new();

            let overall = m.add(common::progress_bar(courses.len() as u64));
            overall.set_message("Courses");

            for course in courses {
                let sids = query!("SELECT sid, course_code FROM sids WHERE course_code = $1 AND sid NOT IN (SELECT sid FROM evaluations)", course.code)
                    .fetch_all(&conn)
                    .await?.into_iter().map(|s| s.sid).collect::<Vec<_>>();
                let pb = m.insert_before(&overall, common::progress_bar(sids.len() as u64));
                pb.println(format!("Found {} sids for {}", sids.len(), course.code));
                let success = save_evals(&conn, &course, sids, &pb).await?;
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
            let course = query_as!(
                Course,
                "SELECT code, name, unit_id FROM courses WHERE code = $1",
                course
            )
            .fetch_one(&conn)
            .await?;
            let sids = query!("SELECT sid, course_code FROM sids WHERE course_code = $1 AND sid NOT IN (SELECT sid FROM evaluations)", course.code).fetch_all(&conn).await?.into_iter().map(|s| s.sid).collect::<Vec<_>>();
            let pb = progress_bar(sids.len() as u64);
            save_evals(&conn, &course, sids, &pb).await?;
            pb.finish();
            println!("Done");
        }
        Commands::Evals {
            command: EvalCommands::Stats,
        } => {
            let evals = query!("SELECT COUNT(*) FROM evaluations")
                .fetch_one(&conn)
                .await?
                .count
                .unwrap_or(0);

            let sections =
                query!("SELECT COUNT(*) FROM sids WHERE sid NOT IN (SELECT sid FROM evaluations)")
                    .fetch_one(&conn)
                    .await?
                    .count
                    .unwrap_or(0);

            println!("{} evals", evals);
            println!("{} sections with no eval", sections);
        }
        Commands::Serve { host } => {
            let app = api::app(conn)?;
            let host = host.unwrap_or("0.0.0.0:3000".to_string());
            let listener = tokio::net::TcpListener::bind(&host).await?;
            axum::serve(listener, app.into_make_service()).await?;
        }
    }

    Ok(())
}
