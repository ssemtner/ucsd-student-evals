use crate::settings;
use anyhow::Result;
use serde::Deserialize;
use tokio::fs;

const FILENAME: &str = "cookies.txt";

pub async fn get_cookies() -> String {
    fs::read_to_string(&FILENAME)
        .await
        .expect("Cookies file not found")
}

#[derive(Deserialize)]
struct CookiesResponse {
    name: String,
    value: String,
}

pub async fn fetch_cookies(token: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}:3000/cookies", settings().base_url))
        .header("Authorization", token);
    let res = res.send().await?;
    let json = res.json::<Vec<CookiesResponse>>().await?;
    let cookies = json.iter().fold(String::new(), |acc, cookie| {
        format!("{}{}={};", acc, cookie.name, cookie.value)
    });
    fs::write(FILENAME, cookies)
        .await
        .expect("Couldn't write to cookies file");

    Ok(())
}
