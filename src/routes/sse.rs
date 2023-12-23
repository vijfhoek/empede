use crate::mpd;

pub async fn sse(_req: tide::Request<()>, sender: tide::sse::Sender) -> tide::Result<()> {
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