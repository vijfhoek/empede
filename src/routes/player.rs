use crate::mpd;
use askama::Template;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "player.html")]
struct PlayerTemplate<'a> {
    song: Option<&'a HashMap<String, String>>,
    name: Option<String>,
    state: &'a str,
    consume: bool,
    random: bool,
    repeat: bool,
    single: bool,
    elapsed: f32,
    duration: f32,
}

pub async fn get_player(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;
    let song = mpd.command("currentsong").await?.into_hashmap();
    let status = mpd.command("status").await?.into_hashmap();

    let elapsed = status
        .get("elapsed")
        .and_then(|e| e.parse().ok())
        .unwrap_or(0.0);
    let duration = status
        .get("duration")
        .and_then(|e| e.parse().ok())
        .unwrap_or(1.0);

    let mut template = PlayerTemplate {
        song: if song.is_empty() { None } else { Some(&song) },
        name: None,
        state: &status["state"],
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

    Ok(template.into())
}
