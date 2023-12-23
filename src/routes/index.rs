use crate::crate_version;
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

pub async fn get_index(_req: tide::Request<()>) -> tide::Result {
    Ok(askama_tide::into_response(&IndexTemplate))
}
