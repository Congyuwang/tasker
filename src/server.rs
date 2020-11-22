use crate::error::Error;
use crate::launchctl::list;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use actix_web::web::Query;
use serde::{Deserialize, Serialize};

#[get("/list_all")]
pub async fn list_all() -> impl Responder {
    let all_list = list("");
    match all_list {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}

#[get("/list/{label}")]
pub async fn list_filter(web::Path((label)): web::Path<(String)>) -> impl Responder {
    let list_result = list(&label);
    match list_result {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}

#[derive(Deserialize)]
struct Label {
    label: String,
}

#[get("/list")]
pub async fn list_param(param: Query<Label>) -> impl Responder {
    let list_result = list(&param.label);
    match list_result {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}
