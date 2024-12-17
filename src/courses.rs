use crate::common;
use crate::database::{Course, Unit};
use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use sqlx::{query, Pool, Postgres};
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

pub async fn display_stats(conn: &Pool<Postgres>) -> Result<()> {
    let unit_count = query!("SELECT COUNT(*) FROM units")
        .fetch_one(conn)
        .await?
        .count
        .unwrap_or(0);

    let course_count = query!("SELECT COUNT(*) FROM courses")
        .fetch_one(conn)
        .await?
        .count
        .unwrap_or(0);

    println!("Units: {}", unit_count);
    println!("Courses: {}", course_count);

    Ok(())
}

pub async fn get_all_courses(conn: &Pool<Postgres>) -> Result<()> {
    let client = common::client()?;

    let units = get_units(&client).await?;
    println!("Found {:?} units", units.len());

    let pb = common::progress_bar(units.len() as u64);
    let mut courses = stream::iter(&units)
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

    let saved = query!(
        "
            INSERT INTO units (id, name)
            SELECT * FROM UNNEST($1::int[], $2::text[])
            ON CONFLICT (id) DO UPDATE
            SET name = EXCLUDED.name
        ",
        &units.iter().map(|u| u.id).collect::<Vec<_>>()[..],
        &units.into_iter().map(|u| u.name).collect::<Vec<_>>()[..]
    )
    .execute(conn)
    .await?
    .rows_affected();

    println!("Saved {} units", saved);

    courses.sort_unstable_by(|a, b| a.code.cmp(&b.code));
    courses.dedup_by(|a, b| a.code == b.code);

    let saved = query!(
        "
            INSERT INTO courses (code, unit_id, name)
            SELECT * FROM UNNEST($1::text[], $2::int[], $3::text[])
            ON CONFLICT (code) DO UPDATE
            SET name = EXCLUDED.name
        ",
        &courses.iter().map(|c| c.code.clone()).collect::<Vec<_>>()[..],
        &courses.iter().map(|c| c.unit_id).collect::<Vec<_>>()[..],
        &courses.into_iter().map(|c| c.name).collect::<Vec<_>>()[..]
    )
    .execute(conn)
    .await?
    .rows_affected();

    println!("Saved {} courses", saved);

    Ok(())
}

async fn get_units(client: &Client) -> Result<Vec<Unit>> {
    let mut body = HashMap::new();
    body.insert("knownCategoryValues", "");
    body.insert("category", "Unit");
    body.insert("contextKey", "UnitID:0");

    let res = client
        .post("https://academicaffairs.ucsd.edu/Modules/Evals/SET/Reports/Search.aspx/GetUnits")
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
    let mut body = HashMap::new();
    body.insert("knownCategoryValues", format!("Unit:{}", unit_id));
    body.insert("category", "Course".to_string());
    body.insert("contextKey", "SubjectCode:;CourseCode:".to_string());

    let res = client
        .post("https://academicaffairs.ucsd.edu/Modules/Evals/SET/Reports/Search.aspx/GetCourses")
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
