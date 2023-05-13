use std::path::Path;

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
    let queue = mpd::playlist()?;
    let template = QueueTemplate { queue };
    Ok(template.into())
}

#[derive(Template)]
#[template(path = "player.html")]
struct PlayerTemplate {
    song: Option<mpdrs::Song>,
    name: Option<String>,
    state: mpdrs::State,
    elapsed: f32,
    duration: f32,
}

async fn get_player(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::connect()?;
    let song = mpd.currentsong()?;
    let status = mpd.status()?;

    let elapsed = status.elapsed.map(|d| d.as_secs_f32()).unwrap_or(0.0);
    let duration = status.duration.map(|d| d.as_secs_f32()).unwrap_or(0.0);

    let mut template = PlayerTemplate {
        song: song.clone(),
        name: None,
        state: status.state,
        elapsed,
        duration,
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
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let entries = mpd::ls(&path)?;

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

    let mut mpd = mpd::connect()?;
    if let Some(id) = query.id {
        mpd.deleteid(id)?;
    } else {
        mpd.clear()?;
    }

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

#[derive(Deserialize, Debug)]
struct UpdateQueueBody {
    from: u32,
    to: u32,
}

async fn post_queue_move(mut req: tide::Request<()>) -> tide::Result {
    let body: UpdateQueueBody = req.body_json().await?;
    let mut mpd = mpd::connect()?;
    mpd.move_range(
        mpdrs::song::Range(Some(body.from), Some(body.from + 1)),
        body.to as usize,
    )?;
    Ok("".into())
}

async fn get_art(req: tide::Request<()>) -> tide::Result {
    let query: IndexQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let resp = if let Ok(art) = mpd::connect()?.albumart(&path) {
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
