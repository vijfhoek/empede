use std::{collections::HashMap, path::Path};

use askama::Template;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

mod mpd;

macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Deserialize, Default)]
#[serde(default)]
struct IndexQuery {
    path: String,
}

async fn get_index(_req: tide::Request<()>) -> tide::Result {
    Ok(askama_tide::into_response(&IndexTemplate))
}

#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    queue: Vec<mpd::QueueItem>,
}

async fn get_queue(_req: tide::Request<()>) -> tide::Result {
    let queue = mpd::Mpd::connect().await?.playlist().await?;
    let template = QueueTemplate { queue };
    Ok(template.into())
}

#[derive(Template)]
#[template(path = "player.html")]
struct PlayerTemplate<'a> {
    song: Option<&'a HashMap<String, String>>,
    name: Option<String>,
    state: &'a str,
    elapsed: f32,
    duration: f32,
}

async fn get_player(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::Mpd::connect().await?;
    let song = mpd.command("currentsong").await?.into_hashmap();
    let status = mpd.command("status").await?.into_hashmap();

    let elapsed = status["elapsed"].parse().unwrap_or(0.0);
    let duration = status["duration"].parse().unwrap_or(1.0);

    let mut template = PlayerTemplate {
        song: if song.is_empty() { None } else { Some(&song) },
        name: None,
        state: &status["state"],
        elapsed,
        duration,
    };

    if !song.is_empty() {
        let name = song.get("Title").unwrap_or(&song["file"]).to_string();
        template.name = Some(name);
    }

    Ok(template.into())
}

#[derive(Template)]
#[template(path = "browser.html")]
struct BrowserTemplate {
    path: Vec<String>,
    entries: Vec<mpd::Entry>,
}

async fn get_browser(req: tide::Request<()>) -> tide::Result {
    let query: IndexQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let entries = mpd::Mpd::connect().await?.ls(&path).await?;

    let template = BrowserTemplate {
        path: Path::new(&*path)
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect(),
        entries,
    };

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

async fn post_queue(req: tide::Request<()>) -> tide::Result {
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

async fn delete_queue(req: tide::Request<()>) -> tide::Result {
    let query: DeleteQueueQuery = req.query()?;

    let mut mpd = mpd::Mpd::connect().await?;
    if let Some(id) = query.id {
        mpd.command(&format!("deleteid {id}")).await?;
    } else {
        mpd.command("clear").await?;
    }

    Ok("".into())
}

async fn post_play(_req: tide::Request<()>) -> tide::Result {
    mpd::Mpd::connect().await?.command("play").await?;
    Ok("".into())
}

async fn post_pause(_req: tide::Request<()>) -> tide::Result {
    mpd::Mpd::connect().await?.command("pause 1").await?;
    Ok("".into())
}

async fn post_previous(_req: tide::Request<()>) -> tide::Result {
    mpd::Mpd::connect().await?.command("previous").await?;
    Ok("".into())
}

async fn post_next(_req: tide::Request<()>) -> tide::Result {
    mpd::Mpd::connect().await?.command("next").await?;
    Ok("".into())
}

#[derive(Deserialize, Debug)]
struct UpdateQueueBody {
    from: u32,
    to: u32,
}

async fn post_queue_move(mut req: tide::Request<()>) -> tide::Result {
    let body: UpdateQueueBody = req.body_json().await?;
    let mut mpd = mpd::Mpd::connect().await?;
    mpd.command(&format!("move {} {}", body.from, body.to))
        .await?;
    Ok("".into())
}

async fn get_art(req: tide::Request<()>) -> tide::Result {
    let query: IndexQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();

    let mut mpd = mpd::Mpd::connect().await?;

    let resp = if let Ok(art) = mpd.albumart(&path).await {
        let mime = infer::get(&art)
            .map(|k| k.mime_type())
            .unwrap_or("application/octet-stream");

        tide::Response::builder(tide::StatusCode::Ok)
            .body(art)
            .content_type(mime)
            .header("cache-control", "max-age=3600")
    } else if let Ok(art) = mpd.readpicture(&path).await {
        let mime = infer::get(&art)
            .map(|k| k.mime_type())
            .unwrap_or("application/octet-stream");

        tide::Response::builder(tide::StatusCode::Ok)
            .body(art)
            .content_type(mime)
            .header("cache-control", "max-age=3600")
    } else {
        tide::Response::builder(tide::StatusCode::NotFound)
    };

    Ok(resp.into())
}

async fn sse(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    // Update everything on connect
    sender.send("playlist", "", None).await?;
    sender.send("player", "", None).await?;

    let mut mpd = mpd::Mpd::connect().await?;

    loop {
        let systems = mpd.idle(&["playlist", "player", "database"]).await?;
        for system in systems {
            sender.send(&system, "", None).await?;
        }
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    let mut app = tide::new();
    app.with(tide_tracing::TraceMiddleware::new());

    app.at("/").get(get_index);
    app.at("/queue").get(get_queue);
    app.at("/player").get(get_player);
    app.at("/art").get(get_art);
    app.at("/browser").get(get_browser);

    app.at("/sse").get(tide::sse::endpoint(sse));

    app.at("/queue").post(post_queue);
    app.at("/queue").delete(delete_queue);
    app.at("/queue/move").post(post_queue_move);

    app.at("/play").post(post_play);
    app.at("/pause").post(post_pause);
    app.at("/previous").post(post_previous);
    app.at("/next").post(post_next);

    app.at("/static").serve_dir("static/")?;

    let bind = std::env::var("EMPEDE_BIND").unwrap_or("0.0.0.0:8080".to_string());
    app.listen(bind).await?;

    Ok(())
}
