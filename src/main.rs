mod mpd;
mod routes;
mod crate_version;

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

    app.at("/").get(routes::index::get_index);
    app.at("/player").get(routes::player::get_player);
    app.at("/browser").get(routes::browser::get_browser);
    app.at("/art").get(routes::art::get_art);

    app.at("/sse").get(tide::sse::endpoint(sse));

    app.at("/queue").get(routes::queue::get_queue);
    app.at("/queue").post(routes::queue::post_queue);
    app.at("/queue").delete(routes::queue::delete_queue);
    app.at("/queue/move").post(routes::queue::post_queue_move);

    app.at("/play").post(post_play);
    app.at("/pause").post(post_pause);
    app.at("/previous").post(post_previous);
    app.at("/next").post(post_next);

    app.at("/static").serve_dir("static/")?;

    let bind = std::env::var("EMPEDE_BIND").unwrap_or("0.0.0.0:8080".to_string());
    app.listen(bind).await?;

    Ok(())
}
