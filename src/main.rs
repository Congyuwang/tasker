use actix_web::{middleware, web, App, HttpServer};
use tasker::server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(server::list_all)
            .service(server::list_filter)
            .service(server::list_param)
            .service(
                web::resource("/")
                    .route(web::get().to(server::index))
                    .route(web::post().to(server::create_new_tasks)),
            )
    })
    .bind("127.0.0.1:54321")?
    .run()
    .await
}
