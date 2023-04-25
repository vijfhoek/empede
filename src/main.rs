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
    let entries = mpd::ls(&query.path)?;
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
    let queue = mpd::playlist()?;
    let template = QueueTemplate { queue };
    Ok(template.into())
}

#[derive(Deserialize)]
struct PostQueueQuery {
    path: String,
}

async fn post_queue(req: tide::Request<()>) -> tide::Result {
    let query: PostQueueQuery = req.query()?;
    mpd::connect()?.add(&query.path)?;
    Ok("".into())
}

async fn post_play(_req: tide::Request<()>) -> tide::Result {
    mpd::connect()?.play()?;
    Ok("".into())
}

async fn post_pause(_req: tide::Request<()>) -> tide::Result {
    mpd::connect()?.pause(true)?;
    Ok("".into())
}

async fn post_previous(_req: tide::Request<()>) -> tide::Result {
    mpd::connect()?.prev()?;
    Ok("".into())
}

async fn post_next(_req: tide::Request<()>) -> tide::Result {
    mpd::connect()?.next()?;
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
        let (_, changed) = buffer
            .trim_end()
            .split_once(": ")
            .ok_or(anyhow!("unexpected response from MPD"))?;
        sender.send(changed, "", None).await?;

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

    app.at("/sse").get(tide::sse::endpoint(sse));

    app.at("/queue").post(post_queue);
    app.at("/play").post(post_play);
    app.at("/pause").post(post_pause);
    app.at("/previous").post(post_previous);
    app.at("/next").post(post_next);

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
