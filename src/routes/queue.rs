use askama::Template;
use crate::mpd;
use serde::Deserialize;
use percent_encoding::percent_decode_str;

#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    queue: Vec<mpd::QueueItem>,
}

pub async fn get_queue(_req: tide::Request<()>) -> tide::Result {
    let queue = mpd::Mpd::connect().await?.playlist().await?;
    let template = QueueTemplate { queue };
    Ok(template.into())
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

pub async fn post_queue(req: tide::Request<()>) -> tide::Result {
    let query: PostQueueQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let mut mpd = mpd::Mpd::connect().await?;

    if query.replace {
        mpd.clear().await?;
    }

    if query.next {
        mpd.add_pos(&path, "+0").await?;
    } else {
        mpd.add(&path).await?;
    }

    if query.play {
        mpd.play().await?;
    }

    Ok("".into())
}

#[derive(Deserialize)]
struct DeleteQueueQuery {
    #[serde(default)]
    id: Option<u32>,
}

pub async fn delete_queue(req: tide::Request<()>) -> tide::Result {
    let query: DeleteQueueQuery = req.query()?;

    let mut mpd = mpd::Mpd::connect().await?;
    if let Some(id) = query.id {
        mpd.command(&format!("deleteid {id}")).await?;
    } else {
        mpd.command("clear").await?;
    }

    Ok("".into())
}

#[derive(Deserialize, Debug)]
struct UpdateQueueBody {
    from: u32,
    to: u32,
}

pub async fn post_queue_move(mut req: tide::Request<()>) -> tide::Result {
    let body: UpdateQueueBody = req.body_json().await?;
    let mut mpd = mpd::Mpd::connect().await?;
    mpd.command(&format!("move {} {}", body.from, body.to))
        .await?;
    Ok("".into())
}
