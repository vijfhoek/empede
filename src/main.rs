use std::path::Path;

use askama::Template;
use serde::Deserialize;

mod mpd;

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
    let queue = mpd::playlist()?;
    let template = QueueTemplate { queue };
    Ok(template.into())
}

#[derive(Template)]
#[template(path = "player.html")]
struct CurrentTemplate {
    song: Option<mpdrs::Song>,
    name: Option<String>,
}

async fn get_player(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::connect()?;
    let song = mpd.currentsong()?;

    let mut template = CurrentTemplate {
        song: song.clone(),
        name: None,
    };

    if let Some(song) = song {
        let name = song.title.unwrap_or(song.file);
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
    let entries = mpd::ls(&query.path)?;

    let template = BrowserTemplate {
        path: Path::new(&query.path)
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
}

async fn post_queue(req: tide::Request<()>) -> tide::Result {
    let query: PostQueueQuery = req.query()?;
    mpd::connect()?.add(&query.path)?;
    Ok("".into())
}

async fn delete_queue(_req: tide::Request<()>) -> tide::Result {
    mpd::connect()?.clear()?;
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

async fn get_art(req: tide::Request<()>) -> tide::Result {
    let query: IndexQuery = req.query()?;
    let resp = if let Ok(art) = mpd::connect()?.albumart(&query.path) {
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

    loop {
        let systems = mpd::idle(&["playlist", "player", "database"]).await?;
        for system in systems {
            sender.send(&system, "", None).await?;
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

    app.at("/").get(get_index);
    app.at("/queue").get(get_queue);
    app.at("/player").get(get_player);
    app.at("/art").get(get_art);
    app.at("/browser").get(get_browser);

    app.at("/sse").get(tide::sse::endpoint(sse));

    app.at("/queue").post(post_queue);
    app.at("/queue").delete(delete_queue);

    app.at("/play").post(post_play);
    app.at("/pause").post(post_pause);
    app.at("/previous").post(post_previous);
    app.at("/next").post(post_next);

    app.at("/static").serve_dir("static/")?;

    let bind = std::env::var("EMPEDE_BIND").unwrap_or("0.0.0.0:8080".to_string());
    app.listen(bind).await?;

    Ok(())
}
