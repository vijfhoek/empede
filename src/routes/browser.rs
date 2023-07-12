use askama::Template;
use serde::Deserialize;
use crate::mpd;
use percent_encoding::percent_decode_str;
use std::path::Path;

#[derive(Template)]
#[template(path = "browser.html")]
struct BrowserTemplate {
    path: Vec<String>,
    entries: Vec<mpd::Entry>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BrowserQuery {
    path: String,
}

pub async fn get_browser(req: tide::Request<()>) -> tide::Result {
    let query: BrowserQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let entries = mpd::Mpd::connect().await?.ls(&path).await?;

    let template = BrowserTemplate {
        path: Path::new(&*path)
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect(),
        entries,
    };

    Ok(template.into())
}
