use actix_web::dev::ServiceRequest;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use tasker::{initialize::get_environment, server};

async fn validator(
    req: ServiceRequest,
    _credentials: BasicAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    if _credentials.user_id().eq("influxdb")
        && _credentials.password().is_some()
        && _credentials.password().unwrap().eq("influxdb_admin")
    {
        Ok(req)
    } else {
        Err(actix_web::Error::from(HttpResponse::Forbidden()))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app = HttpServer::new(|| {
        let auth = HttpAuthentication::basic(validator);
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(auth)
            .service(server::list_all)
            .service(server::list_param)
            .service(server::delete_param)
            .service(server::load_param)
            .service(server::unload_param)
            .service(
                web::resource("/")
                    .route(web::get().to(server::index))
                    .route(web::post().to(server::create_new_tasks)),
            )
    });

    let env = get_environment().unwrap();
    if let (Some(pk), Some(crt)) = (&env.pk_dir, &env.crt_dir) {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(pk, SslFiletype::PEM)
            .expect("private ssl key error");
        builder
            .set_certificate_chain_file(crt)
            .expect("ssl crt file error");
        app.bind_openssl(get_environment().unwrap().address(), builder)?
            .workers(2)
            .run()
            .await
    } else {
        app.bind(get_environment().unwrap().address())?
            .workers(2)
            .run()
            .await
    }
}
