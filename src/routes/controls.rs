use actix_web::{post, HttpResponse, Responder};

use crate::mpd;

async fn toggle_setting(setting: &str) -> anyhow::Result<()> {
    let mut mpd = mpd::get_instance().await;

    let status = mpd.command("status").await?.into_hashmap();
    let value = status[setting] == "1";

    mpd.command(&format!("{} {}", setting, if value { 0 } else { 1 }))
        .await?;
    Ok(())
}

#[post("/play")]
pub async fn post_play() -> impl Responder {
    mpd::command("play").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/pause")]
pub async fn post_pause() -> impl Responder {
    mpd::command("pause 1").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/previous")]
pub async fn post_previous() -> impl Responder {
    mpd::command("previous").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/next")]
pub async fn post_next() -> impl Responder {
    mpd::command("next").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/consume")]
pub async fn post_consume() -> impl Responder {
    toggle_setting("consume").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/random")]
pub async fn post_random() -> impl Responder {
    toggle_setting("random").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/repeat")]
pub async fn post_repeat() -> impl Responder {
    toggle_setting("repeat").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/shuffle")]
pub async fn post_shuffle() -> impl Responder {
    mpd::command("shuffle").await.unwrap();
    HttpResponse::NoContent()
}

#[post("/single")]
pub async fn post_single() -> impl Responder {
    toggle_setting("single").await.unwrap();
    HttpResponse::NoContent()
}
