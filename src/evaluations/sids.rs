use crate::common;
use crate::database::{Course, SectionId};
use anyhow::{anyhow, Result};
use futures::{stream, StreamExt};
use regex::Regex;
use reqwest::Client;
use sqlx::{query, query_as, Pool, Postgres};
use tokio::time::Instant;

pub async fn save_all_sids(conn: &Pool<Postgres>) -> Result<()> {
    let courses = query_as!(Course, "SELECT code, name, unit_id FROM courses")
        .fetch_all(conn)
        .await?;

    let client = common::client()?;

    let pb = common::progress_bar(courses.len() as u64);
    let results = stream::iter(&courses)
        .map(|course| {
            let client = &client;
            let pb = &pb;
            async move {
                let start = Instant::now();
                let res = get_sids(client, course).await;
                match &res {
                    Ok(sids) => {
                        pb.println(format!(
                            "[+] found {} SIDs for {} in {:?}",
                            sids.len(),
                            course.name,
                            start.elapsed()
                        ));
                    }
                    Err(_) => {
                        pb.println(format!(
                            "[-] error in {}: adding to retry queue",
                            course.name
                        ));
                    }
                };
                pb.inc(1);
                (course, res)
            }
        })
        .buffer_unordered(20)
        .collect::<Vec<_>>()
        .await;

    pb.finish();

    let (sids, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(|(_, res)| res.is_ok());
    let mut sids = sids
        .into_iter()
        .flat_map(|(course, res)| res.unwrap().into_iter().map(move |sid| (course, sid)))
        .collect::<Vec<_>>();
    let errors = errors
        .into_iter()
        .map(|(course, _)| course)
        .collect::<Vec<_>>();

    println!("Found {} SIDs with {} errors", sids.len(), errors.len());

    if !errors.is_empty() {
        let mut problems = errors;

        let pb = common::progress_bar(problems.len() as u64);

        while !problems.is_empty() {
            // try to fix them
            let mut fixed = Vec::new();
            for (i, &course) in problems.iter().enumerate() {
                if let Ok(results) = get_sids(&client, course).await {
                    fixed.push(i);
                    sids.extend(results.into_iter().map(|sid| (course, sid)));
                    pb.println(format!("[+] fixed {}", course.name));
                    pb.inc(1);
                }
            }
            for i in fixed.into_iter().rev() {
                problems.remove(i);
            }
        }
        pb.finish();
        println!("Reduced to {} errors", problems.len());

        println!("Now at {} SIDs", sids.len());
    }

    let values = sids
        .clone()
        .into_iter()
        .map(|(course, sid)| SectionId {
            sid,
            course_code: course.code.clone(),
        })
        .collect::<Vec<_>>();

    let saved = query!(
        "
            INSERT INTO sids (sid, course_code)
            SELECT * FROM UNNEST($1::int[], $2::text[])
        ",
        &values.iter().map(|s| s.sid).collect::<Vec<_>>()[..],
        &values
            .into_iter()
            .map(|s| s.course_code)
            .collect::<Vec<_>>()[..]
    )
    .execute(conn)
    .await?
    .rows_affected();

    println!("{saved} SIDs saved");

    Ok(())
}

async fn get_sids(client: &Client, course: &Course) -> Result<Vec<i32>> {
    let res = client
        .post("https://academicaffairs.ucsd.edu/Modules/Evals/SET/Reports/Search.aspx")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("__EVENTTARGET", "".to_string()),
            ("ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$ddlUnit", course.unit_id.to_string()),
            (
                "ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$CascadingDropDown4_ClientState",
                format!("{}:::{}", course.code, course.name.replace(" ", "+"))
            ),
            ("ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$btnSubmit", "Search".to_string())
        ]).send().await?;
    let text = res.text().await?;

    let re = Regex::new(r#"window\.open\('SETSummary\.aspx\?sid=([0-9]*?)',"#)?;
    let res = re
        .captures_iter(&text)
        .map(|c| {
            c.get(1)
                .ok_or(anyhow!("Match had no groups"))
                .map(|m| m.as_str().parse::<i32>().unwrap())
        })
        .collect::<Result<Vec<_>>>();

    res
}
