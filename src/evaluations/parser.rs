use crate::common;
use crate::database::Course;
use crate::evaluations::{get_or_create_instructor_id, get_or_create_term_id};
use anyhow::{anyhow, Result};
use indicatif::ProgressBar;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::{query, Pool, Postgres};
use std::ops::Range;
use tokio::time::Instant;

pub async fn save_evals(
    conn: &Pool<Postgres>,
    course: &Course,
    sids: Vec<i32>,
    pb: &ProgressBar,
) -> Result<bool> {
    let client = common::client()?;

    let mut saved = 0;
    let mut failures = Vec::new();

    pb.set_length(sids.len() as u64);

    for sid in &sids {
        let start = Instant::now();
        let res = get_eval(&client, *sid, course).await;
        match res {
            Ok(eval) => {
                saved += query!(
                    "
                    INSERT INTO evaluations (
                        sid, section_name, course_code, term_id, instructor_id,
                        enrollment, responses,
                        class_helped_understanding, assignments_helped_understanding, fair_exams,
                        timely_feedback, developed_understanding, engaging, communication,
                        help_opportunities, effective_methods, timeliness, welcoming, materials,
                        hours, expected_grades, actual_grades
                    )
                    VALUES (
                        $1, $2, $3, $4, $5,
                        $6, $7,
                        $8, $9, $10,
                        $11, $12, $13, $14,
                        $15, $16, $17, $18, $19,
                        $20, $21, $22
                    )
                ",
                    eval.sid,
                    eval.section_name,
                    eval.course_code,
                    get_or_create_term_id(conn, eval.term).await?,
                    get_or_create_instructor_id(conn, eval.instructor).await?,
                    eval.enrollment,
                    eval.responses,
                    &eval.class_helped_understanding[..],
                    &eval.assignments_helped_understanding[..],
                    &eval.fair_exams[..],
                    &eval.timely_feedback[..],
                    &eval.developed_understanding[..],
                    &eval.engaging[..],
                    &eval.communication[..],
                    &eval.help_opportunities[..],
                    &eval.effective_methods[..],
                    &eval.timeliness[..],
                    &eval.welcoming[..],
                    &eval.materials[..],
                    &eval.hours[..],
                    &eval.expected_grades[..],
                    &eval.actual_grades[..],
                )
                .execute(conn)
                .await?
                .rows_affected();
            }
            Err(e) => {
                failures.push((sid, e));
            }
        }
        pb.inc(1);
        pb.println(format!("Parsed section {} in {:?}", sid, start.elapsed()));
    }

    if !failures.is_empty() {
        pb.println(format!("{} failures for {}", failures.len(), course.name));
        for failure in &failures {
            pb.println(format!("{:?}", failure));
        }
    }

    pb.println(format!("Saved {} evaluations for {}", saved, course.name));

    Ok(failures.is_empty())
}

async fn get_eval(client: &Client, sid: i32, course: &Course) -> Result<Evaluation> {
    let html = get_eval_html(client, sid).await?;
    let eval = parse(&html, sid, course)?;

    Ok(eval)
}

async fn get_eval_html(client: &Client, sid: i32) -> Result<Html> {
    let url = format!(
        "https://academicaffairs.ucsd.edu/Modules/Evals/SET/Reports/SETSummary.aspx?sid={sid}"
    );
    let res = client.get(url.clone()).send().await?;
    let text = res.text().await?;

    Ok(Html::parse_document(&text))
}

#[derive(Debug)]
struct Evaluation {
    sid: i32,
    section_name: String,
    course_code: String,
    term: String,
    instructor: String,

    enrollment: i32,
    responses: i32,

    class_helped_understanding: Vec<i32>,
    assignments_helped_understanding: Vec<i32>,
    fair_exams: Vec<i32>,
    timely_feedback: Vec<i32>,
    developed_understanding: Vec<i32>,
    engaging: Vec<i32>,
    communication: Vec<i32>,
    help_opportunities: Vec<i32>,
    effective_methods: Vec<i32>,
    timeliness: Vec<i32>,
    welcoming: Vec<i32>,
    materials: Vec<i32>,
    hours: Vec<i32>,
    expected_grades: Vec<i32>,
    actual_grades: Vec<i32>,
}

fn parse(html: &Html, sid: i32, course: &Course) -> Result<Evaluation> {
    let (instructor, (term, section_name)) = {
        let selector =
            Selector::parse("#ContentPlaceHolder1_EvalsContentPlaceHolder_lblSummaryTitle > p")
                .unwrap();
        let mut title = html.select(&selector);
        let mut iter = title
            .next()
            .ok_or(anyhow!("Could not find title"))?
            .children()
            .filter_map(|child| child.value().as_text());
        (
            iter.next()
                .map(|s| s.rmatch_indices(',').nth(1).map(|(i, _)| s.split_at(i + 2)))
                .unwrap_or(None)
                .ok_or(anyhow!("Could not find instructor name"))?
                .1
                .trim(),
            {
                let (first, second) = iter
                    .next()
                    .map(|s| s.split_once(","))
                    .unwrap_or(None)
                    .ok_or(anyhow!("Could not parse term"))?;
                (
                    first.trim(),
                    Regex::new(r"Section ID .*? \((.*?)\)")?
                        .captures(second)
                        .map(|captures| captures.get(1))
                        .unwrap_or(None)
                        .map(|m| m.as_str())
                        .ok_or(anyhow!("No matches for section ID"))?,
                )
            },
        )
    };

    let (responses, enrollment) = {
        let selector = Selector::parse(
            "#ContentPlaceHolder1_EvalsContentPlaceHolder_lblSummaryTitle > p:nth-child(2)",
        )
        .unwrap();
        let mut stats = html
            .select(&selector)
            .next()
            .ok_or(anyhow!("Could not find stats"))?
            .children()
            .filter_map(|child| {
                child
                    .value()
                    .as_text()
                    .map(|text| text.split_once(": "))
                    .unwrap_or(None)
                    .map(|(_, s)| s.trim().parse::<i32>().ok())
                    .unwrap_or(None)
            });
        (
            stats
                .next()
                .ok_or(anyhow!("Could not find response count"))?,
            stats
                .next()
                .ok_or(anyhow!("Could not find enrollment count"))?,
        )
    };

    let expected_grades = parse_grades_table(
        html,
        Selector::parse(
            "#ContentPlaceHolder1_EvalsContentPlaceHolder_tblExpectedGrades > tbody > tr",
        )
        .unwrap(),
    )
    .unwrap_or_default();

    let actual_grades = parse_grades_table(
        html,
        Selector::parse(
            "#ContentPlaceHolder1_EvalsContentPlaceHolder_tblGradesReceived > tbody > tr",
        )
        .unwrap(),
    )?;

    let (hours, materials, scales_range) = parse_hours_materials(html)?;

    let scales = scales_range
        .map(|i| parse_scale::<6>(html, i))
        .collect::<Result<Vec<_>>>()?;

    Ok(Evaluation {
        sid,
        section_name: section_name.to_string(),
        course_code: course.code.clone(),
        term: term.to_string(),
        instructor: instructor.to_string(),
        enrollment,
        responses,
        class_helped_understanding: scales[0].clone(),
        assignments_helped_understanding: scales[1].clone(),
        fair_exams: scales[2].clone(),
        timely_feedback: scales[3].clone(),
        developed_understanding: scales[4].clone(),
        engaging: scales[5].clone(),
        communication: scales[6].clone(),
        help_opportunities: scales[7].clone(),
        effective_methods: scales[8].clone(),
        timeliness: scales[9].clone(),
        welcoming: scales[10].clone(),
        materials,
        hours,
        expected_grades,
        actual_grades,
    })
}

fn parse_hours_materials(html: &Html) -> Result<(Vec<i32>, Vec<i32>, Range<u32>)> {
    for long_hours_idx in 14..=20 {
        if let Ok(hours) = parse_scale::<11>(html, long_hours_idx) {
            return Ok((hours, parse_scale::<5>(html, long_hours_idx - 1)?, 0..11));
        }
    }
    Ok((
        parse_scale::<4>(html, 2)?,
        parse_scale::<5>(html, 1)?,
        4..15,
    ))
}

fn parse_grades_table(html: &Html, selector: Selector) -> Result<Vec<i32>> {
    let td_selector = Selector::parse("td").unwrap();

    html.select(&selector)
        .next()
        .ok_or(anyhow!("Could not find expected grades table"))?
        .select(&td_selector)
        .map(|col| {
            col.text()
                .collect::<String>()
                .trim()
                .parse::<i32>()
                .map_err(|err| anyhow!(err))
        })
        .collect::<Result<Vec<i32>>>()
}

fn parse_scale<const N: usize>(html: &Html, index: u32) -> Result<Vec<i32>> {
    let mut result = vec![0; N];
    for (i, item) in result.iter_mut().enumerate() {
        let selector = Selector::parse(&format!("#ContentPlaceHolder1_EvalsContentPlaceHolder_rptQuestionnaire_rptChoices_{index}_rbSelect_{i}"))
            .map_err(|e| anyhow!("{e:?}"))?;
        let text: String = html
            .select(&selector)
            .next()
            .ok_or(anyhow!("Could not get scale {index} i={i}"))?
            .text()
            .take(1)
            .collect();
        *item = text.parse::<i32>()?;
    }
    Ok(result)
}
