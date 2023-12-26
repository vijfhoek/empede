use crate::crate_version;
use actix_web::{get, Responder};
use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Deserialize, Default)]
#[serde(default)]
struct IndexQuery {
    path: String,
}

#[get("/")]
pub async fn get_index() -> impl Responder {
    IndexTemplate
}
