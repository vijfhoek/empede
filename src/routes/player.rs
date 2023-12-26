use crate::mpd;
use actix_web::{get, Responder};
use askama::Template;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "player.html")]
struct PlayerTemplate {
    song: Option<HashMap<String, String>>,
    name: Option<String>,
    state: String,
    consume: bool,
    random: bool,
    repeat: bool,
    single: bool,
    elapsed: f32,
    duration: f32,
}

#[get("/player")]
pub async fn get_player() -> impl Responder {
    let mut mpd = mpd::get_instance().await;
    let song = mpd.command("currentsong").await.unwrap().into_hashmap();
    let status = mpd.command("status").await.unwrap().into_hashmap();

    let elapsed = status
        .get("elapsed")
        .and_then(|e| e.parse().ok())
        .unwrap_or(0.0);
    let duration = status
        .get("duration")
        .and_then(|e| e.parse().ok())
        .unwrap_or(1.0);

    let mut template = PlayerTemplate {
        song: if song.is_empty() {
            None
        } else {
            Some(song.clone())
        },
        name: None,
        state: status["state"].clone(),
        consume: status["consume"] == "1",
        random: status["random"] == "1",
        repeat: status["repeat"] == "1",
        single: status["single"] == "1",
        elapsed,
        duration,
    };

    if !song.is_empty() {
        let name = song.get("Title").unwrap_or(&song["file"]).to_string();
        template.name = Some(name);
    }

    template
}
