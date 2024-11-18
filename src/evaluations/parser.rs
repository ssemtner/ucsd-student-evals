use crate::common;
use crate::database;
use crate::database::Course;
use crate::evaluations::{get_or_create_instructor_id, get_or_create_term_id};
use crate::schema;
use anyhow::{anyhow, Result};
use diesel::dsl::insert_into;
use diesel::{RunQueryDsl, SqliteConnection};
use futures::StreamExt;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::time::Instant;
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;

pub async fn test(conn: &mut SqliteConnection, sid: i32, course: &Course) -> Result<()> {
    let client = common::client()?;
    let start = Instant::now();
    let html = get_eval_html(&client, sid).await?;
    println!("{:?} to get html", start.elapsed());
    let start = Instant::now();
    let eval = parse(&html, sid, course)?;
    let db_eval = eval.into_database_eval(conn)?;
    insert_into(schema::evaluations::table)
        .values(&db_eval)
        .execute(conn)?;

    println!("{:?}", start.elapsed());
    Ok(())
}

async fn get_eval_html(client: &Client, sid: i32) -> Result<Html> {
    let url = format!(
        "https://academicaffairs.ucsd.edu/Modules/Evals/SET/Reports/SETSummary.aspx?sid={sid}"
    );

    let action = || async {
        println!("trying to fetch");
        let res = client.get(url.clone()).send().await?;
        res.text().await
    };
    let text = Retry::spawn(FixedInterval::from_millis(1000).map(jitter).take(3), action).await?;

    Ok(Html::parse_document(&text))
}

#[derive(Debug)]
enum Hours {
    Short([u32; 4]),
    Long([u32; 11]),
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

    class_helped_understanding: [u32; 6],
    assignments_helped_understanding: [u32; 6],
    fair_exams: [u32; 6],
    timely_feedback: [u32; 6],
    developed_understanding: [u32; 6],
    engaging: [u32; 6],
    communication: [u32; 6],
    help_opportunities: [u32; 6],
    effective_methods: [u32; 6],
    timeliness: [u32; 6],
    welcoming: [u32; 6],
    materials: [u32; 5],
    hours: Hours,
    expected_grades: [u32; 7],
    actual_grades: [u32; 7],
}

impl Evaluation {
    fn into_database_eval(self, conn: &mut SqliteConnection) -> Result<database::Evaluation> {
        Ok(database::Evaluation {
            sid: self.sid,
            section_name: self.section_name,
            course_code: self.course_code,

            term_id: get_or_create_term_id(conn, self.term)?,
            instructor_id: get_or_create_instructor_id(conn, self.instructor)?,

            enrollment: self.enrollment,
            responses: self.responses,

            class_helped_understanding: self.class_helped_understanding.into(),
            assignments_helped_understanding: self.assignments_helped_understanding.into(),
            fair_exams: self.fair_exams.into(),
            timely_feedback: self.timely_feedback.into(),
            developed_understanding: self.developed_understanding.into(),
            engaging: self.engaging.into(),
            communication: self.communication.into(),
            help_opportunities: self.help_opportunities.into(),
            effective_methods: self.effective_methods.into(),
            timeliness: self.timeliness.into(),
            welcoming: self.welcoming.into(),
            materials: self.materials.into(),
            hours: match self.hours {
                Hours::Short(arr) => arr.into(),
                Hours::Long(arr) => arr.into(),
            },
            expected_grades: self.expected_grades.into(),
            actual_grades: self.actual_grades.into(),
        })
    }
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
                .map(|s| s.split_once(&(course.name.clone() + ",")))
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
    )?;

    let actual_grades = parse_grades_table(
        html,
        Selector::parse(
            "#ContentPlaceHolder1_EvalsContentPlaceHolder_tblGradesReceived > tbody > tr",
        )
        .unwrap(),
    )?;

    let (hours, materials, scales_range) = match parse_scale(html, 14) {
        Ok(hours) => (Hours::Long(hours), parse_scale::<5>(html, 13)?, 0..12),
        Err(_) => (
            Hours::Short(parse_scale(html, 2)?),
            parse_scale(html, 1)?,
            4..16,
        ),
    };
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
        class_helped_understanding: scales[0],
        assignments_helped_understanding: scales[1],
        fair_exams: scales[2],
        timely_feedback: scales[3],
        developed_understanding: scales[4],
        engaging: scales[5],
        communication: scales[6],
        help_opportunities: scales[7],
        effective_methods: scales[8],
        timeliness: scales[9],
        welcoming: scales[10],
        materials,
        hours,
        expected_grades,
        actual_grades,
    })
}

fn parse_grades_table(html: &Html, selector: Selector) -> Result<[u32; 7]> {
    let td_selector = Selector::parse("td").unwrap();

    html.select(&selector)
        .next()
        .ok_or(anyhow!("Could not find expected grades table"))?
        .select(&td_selector)
        .map(|col| {
            col.text()
                .collect::<String>()
                .trim()
                .parse::<u32>()
                .map_err(|err| anyhow!(err))
        })
        .collect::<Result<Vec<u32>>>()
        .map(|vec| vec.try_into().unwrap())
}

fn parse_scale<const N: usize>(html: &Html, index: u32) -> Result<[u32; N]> {
    let mut result: [u32; N] = [0; N];
    for i in 0..N {
        let selector = Selector::parse(&format!("#ContentPlaceHolder1_EvalsContentPlaceHolder_rptQuestionnaire_rptChoices_{index}_rbSelect_{i}"))
            .map_err(|e| anyhow!("{e:?}"))?;
        let text: String = html
            .select(&selector)
            .next()
            .ok_or(anyhow!("Could not get scale {index} i={i}"))?
            .text()
            .take(1)
            .collect();
        result[i] = text.parse::<u32>()?;
    }
    Ok(result)
}
