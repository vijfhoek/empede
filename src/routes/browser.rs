use crate::mpd;
use actix_web::{get, web, Responder};
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

#[get("/browser")]
pub async fn get_browser(query: web::Query<BrowserQuery>) -> impl Responder {
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let mut mpd = mpd::get_instance().await;
    let entries = mpd.ls(&path).await.unwrap();

    BrowserTemplate {
        path: Path::new(&*path)
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect(),
        entries,
    }
}
