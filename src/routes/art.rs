use crate::mpd;
use actix_web::{
    get,
    http::header::{self, CacheDirective},
    web, HttpResponse, Responder,
};
use percent_encoding::percent_decode_str;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct ArtQuery {
    path: String,
}

#[get("/art")]
pub async fn get_art(query: web::Query<ArtQuery>) -> impl Responder {
    let path = percent_decode_str(&query.path).decode_utf8_lossy();
    let mut mpd = mpd::get_instance().await;

    if let Ok(art) = mpd.albumart(&path).await {
        let mime = infer::get(&art)
            .map(|k| k.mime_type())
            .unwrap_or("application/octet-stream");

        HttpResponse::Ok()
            .content_type(mime)
            .append_header(header::CacheControl(vec![CacheDirective::MaxAge(3600)]))
            .body(art)
    } else if let Ok(art) = mpd.readpicture(&path).await {
        let mime = infer::get(&art)
            .map(|k| k.mime_type())
            .unwrap_or("application/octet-stream");

        HttpResponse::Ok()
            .content_type(mime)
            .append_header(header::CacheControl(vec![CacheDirective::MaxAge(3600)]))
            .body(art)
    } else {
        HttpResponse::NotFound().finish()
    }
}
