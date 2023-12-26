use crate::mpd;
use actix_web::{delete, get, post, web, HttpResponse, Responder};
use askama::Template;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    queue: Vec<mpd::QueueItem>,
}

#[get("/queue")]
pub async fn get_queue() -> impl Responder {
    let mut mpd = mpd::get_instance().await;
    let queue = mpd.playlist().await.unwrap();
    QueueTemplate { queue }
}

#[derive(Deserialize)]
struct PostQueueQuery {
    path: String,
    #[serde(default)]
    replace: bool,
    #[serde(default)]
    next: bool,
    #[serde(default)]
    play: bool,
}

#[post("/queue")]
pub async fn post_queue(query: web::Query<PostQueueQuery>) -> impl Responder {
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let mut mpd = mpd::get_instance().await;

    if query.replace {
        mpd.clear().await.unwrap();
    }

    if query.next {
        mpd.add_pos(&path, "+0").await.unwrap();
    } else {
        mpd.add(&path).await.unwrap();
    }

    if query.play {
        mpd.play().await.unwrap();
    }

    HttpResponse::NoContent()
}

#[derive(Deserialize)]
struct DeleteQueueQuery {
    #[serde(default)]
    id: Option<u32>,
}

#[delete("/queue")]
pub async fn delete_queue(query: web::Query<DeleteQueueQuery>) -> impl Responder {
    let mut mpd = mpd::get_instance().await;
    if let Some(id) = query.id {
        mpd.command(&format!("deleteid {id}")).await.unwrap();
    } else {
        mpd.command("clear").await.unwrap();
    }

    HttpResponse::NoContent()
}

#[derive(Deserialize, Debug)]
struct UpdateQueueBody {
    from: u32,
    to: u32,
}

#[post("/queue/move")]
pub async fn post_queue_move(body: web::Json<UpdateQueueBody>) -> impl Responder {
    let mut mpd = mpd::get_instance().await;
    mpd.command(&format!("move {} {}", body.from, body.to))
        .await
        .unwrap();
    HttpResponse::NoContent()
}
