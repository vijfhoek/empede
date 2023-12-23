mod crate_version;
mod mpd;
mod routes;

async fn post_play(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;
    mpd.command("play").await?;
    Ok("".into())
}

async fn post_pause(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;
    mpd.command("pause 1").await?;
    Ok("".into())
}

async fn post_previous(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;
    mpd.command("previous").await?;
    Ok("".into())
}

async fn post_next(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;
    mpd.command("next").await?;
    Ok("".into())
}

async fn post_consume(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;

    let status = mpd.command("status").await?.into_hashmap();
    let consume = status["consume"] == "1";

    mpd.command(&format!("consume {}", if consume { 0 } else { 1 }))
        .await?;
    Ok("".into())
}

async fn post_shuffle(_req: tide::Request<()>) -> tide::Result {
    let mut mpd = mpd::get_instance().await;

    let status = mpd.command("status").await?.into_hashmap();
    let random = status["random"] == "1";

    mpd.command(&format!("random {}", if random { 0 } else { 1 }))
        .await?;

    Ok("".into())
}

async fn sse(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
    // Update everything on connect
    sender.send("playlist", "", None).await?;
    sender.send("player", "", None).await?;

    let mut mpd = mpd::Mpd::new();
    mpd.connect().await.unwrap();

    loop {
        let systems = mpd
            .idle(&["playlist", "player", "database", "options"])
            .await?;
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

    app.at("/consume").post(post_consume);
    app.at("/shuffle").post(post_shuffle);

    app.at("/static").serve_dir("static/")?;

    let bind = std::env::var("EMPEDE_BIND").unwrap_or("0.0.0.0:8080".to_string());
    app.listen(bind).await?;

    Ok(())
}
