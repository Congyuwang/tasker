use actix_web::{web, HttpServer, App, middleware};
use tasker::server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    std::fs::create_dir_all("/Users/congyuwang/Desktop/tmp/").unwrap();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(server::list_all)
            .service(server::list_filter)
            .service(server::list_param)
            .service(
                web::resource("/")
                    .route(web::get().to(server::index))
                    .route(web::post().to(server::save_file))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
