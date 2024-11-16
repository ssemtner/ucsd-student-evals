use crate::common;
use crate::database::Course;
use crate::schema::courses;
use crate::settings;
use anyhow::{anyhow, Result};
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use futures::{stream, StreamExt, TryStreamExt};
use regex::Regex;
use reqwest::Client;
use std::error::Error;
use std::fmt;
use tokio::time::Instant;

pub async fn get_all_sids(conn: &mut SqliteConnection) -> Result<()> {
    let courses = courses::table
        .select(Course::as_select())
        .get_results(conn)?;

    let client = common::client()?;

    let courses = Vec::from(courses.split_at(100).0);
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
                    Err(err) => {
                        pb.println(format!(
                            "[-] error in {}: adding to retry queue",
                            course.name
                        ));
                    }
                };
                pb.inc(1);
                res.map_err(|err| ErrorWrapper {
                    error: err,
                    value: course,
                })
            }
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;

    pb.finish();

    let (sids, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    let mut sids = sids
        .into_iter()
        .flat_map(Result::unwrap)
        .collect::<Vec<_>>();
    let errors = errors
        .into_iter()
        .map(Result::unwrap_err)
        .collect::<Vec<_>>();

    println!("Found {} SIDs with {} errors", sids.len(), errors.len());
    if errors.len() > 0 {
        let mut problems: Vec<Course> = errors
            .into_iter()
            .map(|wrapper| wrapper.value.clone())
            .collect();

        let pb = common::progress_bar(problems.len() as u64);

        let mut prev_count = 0;

        while problems.len() != prev_count {
            prev_count = problems.len();

            // try to fix them
            let mut fixed = Vec::new();
            for (i, course) in problems.iter().enumerate() {
                if let Ok(results) = get_sids(&client, course).await {
                    fixed.push(i);
                    sids.extend(results);
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

    Ok(())
}

async fn get_sids(client: &Client, course: &Course) -> Result<Vec<u32>> {
    let start = Instant::now();
    let res = client
        .post(format!("{}/Modules/Evals/SET/Reports/Search.aspx", settings().base_url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("__EVENTTARGET", "".to_string()),
            ("ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$ddlUnit", course.unit_id.to_string()),
            (
                "ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$CascadingDropDown4_ClientState",
                format!("{}:::{}", course.code, course.name.replace(" ", "+"))
            ),
            ("ctl00$ctl00$ContentPlaceHolder1$EvalsContentPlaceHolder$btnSubmit", "Search".to_string())
        ])
        .send()
        .await?;

    let text = res.text().await?;
    // println!("got text {:?}", start.elapsed());
    let start = Instant::now();

    let re = Regex::new(r#"window\.open\('SETSummary\.aspx\?sid=([0-9]*?)',"#)?;
    let res = re
        .captures_iter(&text)
        .map(|c| {
            c.get(1)
                .ok_or(anyhow!("Match had no groups"))
                .map(|m| m.as_str().parse::<u32>().unwrap())
        })
        .collect::<Result<Vec<u32>>>();

    // println!("got matches {:?}", start.elapsed());
    res
}

#[derive(Debug)]
pub struct ErrorWrapper<T> {
    error: anyhow::Error,
    value: T,
}

impl<T: fmt::Debug> fmt::Display for ErrorWrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ErrorWrapper {{ error: {}, value: {:?} }}",
            self.error, self.value
        )
    }
}

impl<T: fmt::Debug> Error for ErrorWrapper<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.error.as_ref())
    }
}
