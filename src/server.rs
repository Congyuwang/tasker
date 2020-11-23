use crate::error::Error;
use crate::launchctl::list;
use crate::TEMP_FOLDER;
use actix_multipart::Multipart;
use actix_web::middleware::errhandlers::ErrorHandlerResponse::Response;
use actix_web::web::Query;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::io::Write;

static INDEX_HTML: &'static str = include_str!("index.html");
static MB_LIMIT: usize = 20;
static SIZE_LIMIT: usize = MB_LIMIT * 1024 * 1024;

pub fn index() -> HttpResponse {
    HttpResponse::Ok().body(INDEX_HTML)
}

pub async fn upload(mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {
    let mut size: usize = 0;
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        let filepath = format!("{} {}", TEMP_FOLDER, sanitize_filename::sanitize(&filename));

        // File::create is blocking operation, use thread-pool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            size += data.len();
            if size > SIZE_LIMIT {
                return Err(actix_web::Error::from(
                    HttpResponse::Forbidden()
                        .body(format!("size too big: exceeds {} mb", MB_LIMIT)),
                ));
            }
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }
    Ok(HttpResponse::Ok().into())
}

#[derive(Deserialize)]
pub struct Label {
    label: String,
}

#[get("/list_all")]
pub async fn list_all() -> impl Responder {
    let all_list = list("");
    match all_list {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}

#[get("/list/{label}")]
pub async fn list_filter(web::Path(label): web::Path<String>) -> impl Responder {
    let list_result = list(&label);
    match list_result {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}

#[get("/list")]
pub async fn list_param(param: Query<Label>) -> impl Responder {
    let list_result = list(&param.label);
    match list_result {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}
