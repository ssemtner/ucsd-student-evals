use crate::cookies::get_cookies;
use crate::database::{Course, Unit};
use crate::schema::{courses, units};
use crate::settings;
use anyhow::Result;
use diesel::{RunQueryDsl, SqliteConnection};
use futures::{stream, StreamExt};
use indicatif::ProgressBar;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::time::Instant;

#[derive(Deserialize)]
struct ResponseItem {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct ResponseList {
    d: Vec<ResponseItem>,
}

pub async fn get_all_courses(conn: &mut SqliteConnection) -> Result<()> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.3"),
    );
    headers.insert("Cookie", HeaderValue::from_str(&get_cookies().await)?);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let units = get_units(&client).await?;
    println!("Found {:?} units", units.len());

    let pb = ProgressBar::new(units.len() as u64);

    let courses = stream::iter(&units)
        .map(|unit| {
            let client = &client;
            let pb = &pb;
            async move {
                let start = Instant::now();
                let res = get_courses(client, unit.id).await;
                pb.println(format!(
                    "[+] found courses for {} in {:?}",
                    unit.name,
                    start.elapsed()
                ));
                pb.inc(1);
                res
            }
        })
        .buffer_unordered(4)
        .filter_map(|r| async { r.ok() })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    pb.finish();
    println!("Found {:?} courses", courses.len());

    let saved = diesel::replace_into(units::table)
        .values(units)
        .execute(conn)?;

    println!("Saved {} units", saved);

    let saved = diesel::replace_into(courses::table)
        .values(courses)
        .execute(conn)?;

    println!("Saved {} courses", saved);

    Ok(())
}

async fn get_units(client: &Client) -> Result<Vec<Unit>> {
    let mut body = HashMap::new();
    body.insert("knownCategoryValues", "");
    body.insert("category", "Unit");
    body.insert("contextKey", "UnitID:0");

    let res = client
        .post(format!(
            "{}/Modules/Evals/SET/Reports/Search.aspx/GetUnits",
            settings().base_url
        ))
        .json(&body)
        .send()
        .await?;

    Ok(res
        .json::<ResponseList>()
        .await?
        .d
        .iter()
        .filter_map(|item| {
            item.value.parse().ok().map(|id| Unit {
                id,
                name: item.name.clone(),
            })
        })
        .collect::<Vec<Unit>>())
}

async fn get_courses(client: &Client, unit_id: i32) -> Result<Vec<Course>> {
    let start = Instant::now();
    let mut body = HashMap::new();
    body.insert("knownCategoryValues", format!("Unit:{}", unit_id));
    body.insert("category", "Course".to_string());
    body.insert("contextKey", "SubjectCode:;CourseCode:".to_string());

    let res = client
        .post(format!(
            "{}/Modules/Evals/SET/Reports/Search.aspx/GetCourses",
            settings().base_url
        ))
        .json(&body)
        .send()
        .await?;
    let res = res
        .json::<ResponseList>()
        .await?
        .d
        .into_iter()
        .map(|item| Course {
            code: item.value,
            name: item.name,
            unit_id,
        })
        .collect::<Vec<Course>>();
    Ok(res)
}
