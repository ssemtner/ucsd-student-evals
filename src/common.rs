use crate::cookies::get_cookies;
use crate::settings;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::header::HeaderValue;
use reqwest::{Client, Proxy};
use std::fmt::Write;

pub fn progress_bar(max: u64) -> ProgressBar {
    let pb = ProgressBar::new(max);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        })
        .progress_chars("#>-"),
    );
    pb
}

pub fn client() -> reqwest::Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.3"),
    );
    headers.insert("Cookie", HeaderValue::from_str(&get_cookies()).unwrap());

    let mut builder = Client::builder()
        // .timeout(std::time::Duration::from_secs(3))
        .default_headers(headers);
    if let (Some(username), Some(password)) =
        (&settings().proxy_username, &settings().proxy_password)
    {
        builder = builder.proxy(
            Proxy::all(format!("{}:5000", settings().service_url))?.basic_auth(username, password),
        );
    }
    builder.build()
}
