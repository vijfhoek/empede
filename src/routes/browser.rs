use crate::mpd;
use askama::Template;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
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
    let mut mpd = mpd::get_instance().await;
    let entries = mpd.ls(&path).await?;

    let template = BrowserTemplate {
        path: Path::new(&*path)
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect(),
        entries,
    };

    Ok(template.into())
}
