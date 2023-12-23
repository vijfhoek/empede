use crate::mpd;

async fn toggle_setting(setting: &str) -> anyhow::Result<()> {
    let mut mpd = mpd::get_instance().await;

    let status = mpd.command("status").await?.into_hashmap();
    let value = status[setting] == "1";

    mpd.command(&format!("{} {}", setting, if value { 0 } else { 1 }))
        .await?;
    Ok(())
}

pub async fn post_play(_req: tide::Request<()>) -> tide::Result {
    mpd::command("play").await?;
    Ok("".into())
}

pub async fn post_pause(_req: tide::Request<()>) -> tide::Result {
    mpd::command("pause 1").await?;
    Ok("".into())
}

pub async fn post_previous(_req: tide::Request<()>) -> tide::Result {
    mpd::command("previous").await?;
    Ok("".into())
}

pub async fn post_next(_req: tide::Request<()>) -> tide::Result {
    mpd::command("next").await?;
    Ok("".into())
}

pub async fn post_consume(_req: tide::Request<()>) -> tide::Result {
    toggle_setting("consume").await?;
    Ok("".into())
}

pub async fn post_random(_req: tide::Request<()>) -> tide::Result {
    toggle_setting("random").await?;
    Ok("".into())
}

pub async fn post_repeat(_req: tide::Request<()>) -> tide::Result {
    toggle_setting("repeat").await?;
    Ok("".into())
}

pub async fn post_shuffle(_req: tide::Request<()>) -> tide::Result {
    mpd::command("shuffle").await?;
    Ok("".into())
}

pub async fn post_single(_req: tide::Request<()>) -> tide::Result {
    toggle_setting("single").await?;
    Ok("".into())
}