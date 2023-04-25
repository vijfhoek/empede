use std::path::Path;

use anyhow::anyhow;
use askama::Template;
use async_std::prelude::*;
use async_std::{
    io::{BufReader, WriteExt},
    net::TcpStream,
};
use serde::Deserialize;

mod mpd;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    path: Vec<String>,
    entries: Vec<mpd::Entry>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct IndexQuery {
    path: String,
}

async fn index(req: tide::Request<()>) -> tide::Result {
    let query: IndexQuery = req.query()?;
    let entries = mpd::ls(&query.path).await?;
    let template = IndexTemplate {
        path: Path::new(&query.path)
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect(),
        entries,
    };
    Ok(askama_tide::into_response(&template))
}

#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    queue: Vec<mpd::QueueItem>,
}

async fn get_queue(_req: tide::Request<()>) -> tide::Result {
    let queue = mpd::playlist().await?;
    let template = QueueTemplate { queue };
    Ok(template.into())
}

#[derive(Deserialize)]
struct PostQueueQuery {
    path: String,
}

async fn post_queue(req: tide::Request<()>) -> tide::Result {
    let mut client = mpdrs::Client::connect(mpd::HOST)?;
    let query: PostQueueQuery = req.query()?;
    client.add(&query.path)?;
    Ok("".into())
}

async fn post_play(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpdrs::Client::connect(mpd::HOST)?;
    mpd.play()?;
    Ok("".into())
}
async fn post_pause(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpdrs::Client::connect(mpd::HOST)?;
    mpd.pause(true)?;
    Ok("".into())
}
async fn post_previous(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpdrs::Client::connect(mpd::HOST)?;
    mpd.prev()?;
    Ok("".into())
}
async fn post_next(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpdrs::Client::connect(mpd::HOST)?;
    mpd.next()?;
    Ok("".into())
}

async fn sse(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    // Needs to be async and all async mpd libraries suck
    let mut stream = TcpStream::connect(mpd::HOST).await?;
    let mut reader = BufReader::new(stream.clone());

    // skip OK MPD line
    // TODO check if it is indeed OK
    let mut buffer = String::new();
    reader.read_line(&mut buffer).await?;

    loop {
        stream.write_all(b"idle playlist player\n").await?;

        buffer.clear();
        reader.read_line(&mut buffer).await?;
        if buffer == "changed: playlist\n" {
            sender.send("queue", "", None).await?;
        } else if buffer == "changed: player\n" {
            sender.send("player", "", None).await?;
        }

        buffer.clear();
        reader.read_line(&mut buffer).await?;
        if buffer != "OK\n" {
            Err(anyhow!("mpd didn't return OK"))?;
        }
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut app = tide::new();
    app.with(tide_tracing::TraceMiddleware::new());
    app.at("/").get(index);
    app.at("/queue").get(get_queue);
    app.at("/queue").post(post_queue);
    app.at("/play").post(post_play);
    app.at("/pause").post(post_pause);
    app.at("/previous").post(post_previous);
    app.at("/next").post(post_next);
    app.at("/sse").get(tide::sse::endpoint(sse));
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
