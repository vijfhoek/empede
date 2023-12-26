use actix_web::{middleware::Logger, web, App, HttpServer};

mod crate_version;
mod mpd;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let bind = std::env::var("EMPEDE_BIND").unwrap_or("0.0.0.0:8080".into());
    let (host, port) = bind.split_once(':').unwrap();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(|| {
        App::new().wrap(Logger::default()).service(
            web::scope("")
                .service(routes::index::get_index)
                .service(routes::player::get_player)
                .service(routes::browser::get_browser)
                .service(routes::art::get_art)
                .service(routes::sse::idle)
                .service(routes::queue::get_queue)
                .service(routes::queue::post_queue)
                .service(routes::queue::delete_queue)
                .service(routes::queue::post_queue_move)
                .service(routes::controls::post_play)
                .service(routes::controls::post_pause)
                .service(routes::controls::post_previous)
                .service(routes::controls::post_next)
                .service(routes::controls::post_consume)
                .service(routes::controls::post_random)
                .service(routes::controls::post_repeat)
                .service(routes::controls::post_single)
                .service(routes::controls::post_shuffle)
                .service(actix_files::Files::new("/static", "./static")),
        )
    })
    .bind((host, port.parse().unwrap()))?
    .run()
    .await?;

    Ok(())
}
