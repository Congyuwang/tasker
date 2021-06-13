use crate::error::Error;
use crate::launchctl::{
    create_task, delete_task, get_zip, list, load_task, unload_task, update_yaml, view_std_err,
    view_std_out, view_yaml,
};
use actix_files::NamedFile;
use actix_multipart::{Field, Multipart};
use actix_web::body::Body;
use actix_web::http::StatusCode;
use actix_web::web::Query;
use actix_web::{get, post, web, HttpResponse, Responder};
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use std::io::Write;
use std::path::Path;

static INDEX_HTML: &'static str = include_str!("index.html");
static LIST_ALL_HTML: &'static str = include_str!("list_all.html");
static LIST_PART_HTML: &'static str = include_str!("list_part.html");
static CREATE_SUCCESS: &'static str = include_str!("create_success.html");
static EDIT_YAML: &'static str = include_str!("edit_yaml.html");
static STDOUT: &'static str = include_str!("stdout.html");
static STDERR: &'static str = include_str!("stderr.html");
static MB_LIMIT: usize = 20;
static SIZE_LIMIT: usize = MB_LIMIT * 1024 * 1024;
static TEMP_ZIP: &str = "/tmp/tasker.task.temp.zip";

pub fn index() -> HttpResponse {
    HttpResponse::Ok().body(INDEX_HTML)
}

pub fn list_all() -> HttpResponse {
    HttpResponse::Ok().body(LIST_ALL_HTML)
}

pub fn list_part() -> HttpResponse {
    HttpResponse::Ok().body(LIST_PART_HTML)
}

pub fn create_success() -> HttpResponse {
    HttpResponse::Ok().body(CREATE_SUCCESS)
}

pub fn edit_yaml() -> HttpResponse {
    HttpResponse::Ok().body(EDIT_YAML)
}

pub fn stderr() -> HttpResponse {
    HttpResponse::Ok().body(STDERR)
}

pub fn stdout() -> HttpResponse {
    HttpResponse::Ok().body(STDOUT)
}

///
/// upload file with a size_limit of SIZE_LIMIT bytes for single files
///
pub async fn create_new_tasks(mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        if !filename.ends_with(".zip") {
            let response = HttpResponse::new(StatusCode::BAD_REQUEST);
            return Ok(response.set_body(Body::from("not a zip file")));
        }
        let filepath = Path::new(TEMP_ZIP);
        save_single_zip(&mut field, filename).await?;
        match create_task(filepath) {
            Ok(_) => {}
            Err(e) => {
                let response = HttpResponse::new(StatusCode::BAD_REQUEST);
                return Ok(response.set_body(Body::from(format!("fail to create task: {:?}", e))));
            }
        };
    }
    Ok(create_success())
}

///
/// this function saves the zip to TEMP_ZIP location
///
async fn save_single_zip(field: &mut Field, filename: &str) -> Result<(), actix_web::Error> {
    // File::create is blocking operation, use thread-pool
    let mut f = web::block(|| std::fs::File::create(TEMP_ZIP))
        .await
        .unwrap();

    let mut size: usize = 0;
    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        size += data.len();
        if size > SIZE_LIMIT {
            return Err(actix_web::Error::from(HttpResponse::Forbidden().body(
                format!("{} size too big: exceeds {} mb", filename, MB_LIMIT),
            )));
        }
        f = web::block(move || f.write_all(&data).map(|_| f)).await?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct Label {
    label: String,
}

#[derive(Deserialize)]
pub struct OutputLimited {
    label: String,
    limit: usize,
}

#[get("/list_raw_json")]
pub async fn list_raw_json(param: Query<Label>) -> impl Responder {
    let list_result = list(&param.label);
    match list_result {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
}

#[get("/delete")]
pub async fn delete_param(param: Query<Label>) -> impl Responder {
    let delete_result = delete_task(&param.label);
    match delete_result {
        Ok(_) => HttpResponse::Ok().body("Successfully deleted task"),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

#[get("/load")]
pub async fn load_param(param: Query<Label>) -> impl Responder {
    let load_task = load_task(&param.label);
    match load_task {
        Ok(_) => HttpResponse::Ok().body("Successfully loaded task"),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

#[get("/unload")]
pub async fn unload_param(param: Query<Label>) -> impl Responder {
    let unload_task = unload_task(&param.label);
    match unload_task {
        Ok(_) => HttpResponse::Ok().body("Successfully unloaded task"),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

fn plain_text_response(s: Result<String, Error>) -> impl Responder {
    match s {
        Ok(s) => HttpResponse::Ok().body(s.replace("\n", "<br>")),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

#[get("/stdout_raw")]
pub async fn stdout_param(param: Query<OutputLimited>) -> impl Responder {
    let out = view_std_out(&param.label, param.limit);
    plain_text_response(out)
}

#[get("/stderr_raw")]
pub async fn stderr_param(param: Query<OutputLimited>) -> impl Responder {
    let err = view_std_err(&param.label, param.limit);
    plain_text_response(err)
}

#[get("/get_yaml")]
pub async fn get_yaml(param: Query<Label>) -> impl Responder {
    let yaml = view_yaml(&param.label);
    match yaml {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

#[post("/post_yaml")]
pub async fn post_yaml(body: String, param: Query<Label>) -> impl Responder {
    let result = update_yaml(&body, &param.label);
    match result {
        Ok(_) => HttpResponse::Ok().body("Successfully updated yaml"),
        Err(e) => HttpResponse::BadRequest().body(format!("{:?}", e)),
    }
}

#[get("/get_task_zip")]
pub async fn get_task_zip(param: Query<Label>) -> actix_web::Result<NamedFile> {
    let result = get_zip(&param.label);
    match result {
        Ok(p) => Ok(NamedFile::open(p)?),
        Err(e) => Err(actix_web::Error::from(
            HttpResponse::BadRequest().body(format!("{:?}", e)),
        )),
    }
}
