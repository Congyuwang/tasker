use actix_web::{web, HttpServer, App};
use tasker::server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(server::list_all)
            .service(server::list_filter)
            .service(server::list_param)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
