use crate::error::Error;
use crate::utils;
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct Env {
    domain: String,
    port: u16,
    pub tasker_root: PathBuf,
    pub meta_dir: PathBuf,
    pub meta_file: PathBuf,
    pub trash_dir: PathBuf,
    pub task_dir: PathBuf,
    pub out_dir: PathBuf,
    pub pk_dir: Option<PathBuf>,
    pub crt_dir: Option<PathBuf>,
    pub user_name: String,
    pub password: String,
}

static mut ENVIRONMENT: Option<Env> = None;
static META_FOLDER: &str = "meta";
static TASK_FOLDER: &str = "tasks";
static TRASH_FOLDER: &str = "trash";
static OUT_FOLDER: &str = "out";
static META_FILE: &str = "tasker.meta";
static DOMAIN_RE: &str = "^[A-Za-z0-9]{1,63}(\\.[A-Za-z0-9]{1,63})*$";

pub fn get_environment() -> Option<&'static Env> {
    unsafe {
        if ENVIRONMENT.is_none() {
            ENVIRONMENT = Some(Env::init());
        }
        return ENVIRONMENT.as_ref();
    }
}

impl Env {
    fn init() -> Env {
        std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
        // check or create folders
        let tasker_root = std::env::var("TASKER_ROOT").expect("TASKER_ROOT not found in Env");
        let pk_dir = match std::env::var("SSL_PRIVATE_KEY") {
            Ok(d) => Some(Path::new(&d).to_owned()),
            Err(_) => None,
        };
        let crt_dir = match std::env::var("SSL_CERTIFICATE") {
            Ok(d) => Some(Path::new(&d).to_owned()),
            Err(_) => None,
        };
        let user_name = match std::env::var("USER_NAME") {
            Ok(d) => {
                if d.len() < 5 {
                    panic!("USER_NAME must be at least 5 characters")
                } else {
                    d
                }
            }
            Err(_) => panic!("USER_NAME missing in env"),
        };
        let password = match std::env::var("PASSWORD") {
            Ok(d) => {
                if d.len() < 12 {
                    panic!("PASSWORD must be at least 12 characters")
                } else {
                    d
                }
            }
            Err(_) => panic!("PASSWORD missing in env"),
        };
        let tasker_root = std::path::Path::new(&tasker_root).to_owned();
        let meta_dir = tasker_root.join(META_FOLDER);
        let trash_dir = tasker_root.join(TRASH_FOLDER);
        let task_dir = tasker_root.join(TASK_FOLDER);
        let out_dir = tasker_root.join(OUT_FOLDER);
        utils::create_dir_check(&tasker_root).expect("failed to create tasker_root");
        utils::create_dir_check(&meta_dir).expect("failed to create meta_dir");
        utils::create_dir_check(&trash_dir).expect("failed to create trash_dir");
        utils::create_dir_check(&task_dir).expect("failed to create task_dir");
        utils::create_dir_check(&out_dir).expect("failed to create out_dir");
        let meta_file = meta_dir.join(META_FILE).to_owned();
        utils::create_file_check(&meta_file).expect("failed to create meta file");

        // check domain and port number
        let domain: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());
        Env::check_domain_name(&domain).unwrap();
        let port: String = std::env::var("PORT").unwrap_or_else(|_| "54321".to_string());
        let port: u16 = port.parse().expect("mis-specified port number");
        if port > 65353 {
            panic!("port number out of range")
        }
        Env {
            domain,
            port,
            tasker_root,
            meta_dir,
            trash_dir,
            task_dir,
            out_dir,
            meta_file,
            pk_dir,
            crt_dir,
            user_name,
            password,
        }
    }

    /// Characters should only be a-z | A-Z | 0-9 and period(.) and dash(-)
    /// The domain name part should not start or end with dash (-) (e.g. -google-.com)
    /// The domain name part should be between 1 and 63 characters long
    fn check_domain_name(domain: &str) -> Result<&str, Error> {
        lazy_static! {
            static ref DOMAIN_REGEX: Regex = Regex::new(DOMAIN_RE).unwrap();
        }
        if DOMAIN_REGEX.is_match(domain) {
            Ok(domain)
        } else {
            Err(Error::IllegalDomainName(
                String::from("'") + domain + "' is illegal.",
            ))
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", &self.domain, &self.port)
    }
}
