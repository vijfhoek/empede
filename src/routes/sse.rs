use std::time::Duration;

use actix_web::{get, Responder};
use actix_web_lab::sse;

use crate::mpd::Mpd;

#[get("/idle")]
pub async fn idle() -> impl Responder {
    let mut mpd = Mpd::new();
    mpd.connect().await.unwrap();

    const SYSTEMS: &[&str] = &["playlist", "player", "database", "options"];

    let (tx, rx) = tokio::sync::mpsc::channel(10);
    for system in SYSTEMS {
        _ = tx
            .send(sse::Data::new("").event(system.to_owned()).into())
            .await;
    }

    actix_web::rt::spawn(async move {
        loop {
            let systems = mpd.idle(SYSTEMS).await.unwrap();

            for system in systems {
                _ = tx.send(sse::Data::new("").event(system).into()).await;
            }
        }
    });

    sse::Sse::from_infallible_receiver(rx).with_retry_duration(Duration::from_secs(10))
}
