use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{middleware, web, App, HttpServer};
use tasker::{initialize::get_environment, server};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let key = "a_very_long_password_for_influxdb_admin"
        .as_bytes()
        .to_owned();

    HttpServer::new(move || {
        App::new()
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&key)
                    .max_age(60 * 20)
                    .name("auth-cookie")
                    .secure(false),
            ))
            .wrap(middleware::Logger::default())
            .service(server::list_all)
            .service(server::list_param)
            .service(server::delete_param)
            .service(
                web::resource("/")
                    .route(web::get().to(server::index))
                    .route(web::post().to(server::create_new_tasks)),
            )
    })
    .bind(get_environment().unwrap().address())?
    .run()
    .await
}
