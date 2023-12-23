use crate::mpd;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct ArtQuery {
    path: String,
}

pub async fn get_art(req: tide::Request<()>) -> tide::Result {
    let query: ArtQuery = req.query()?;
    let path = percent_decode_str(&query.path).decode_utf8_lossy();

    let mut mpd = mpd::get_instance().await;

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
