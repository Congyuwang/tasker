#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum_macros;

static DOMAIN_NAME: &str = "com.tasker.tasks";
static TEMP_FOLDER: &str = "/tmp";

/// the config module provides api to convert task configuration to and from yaml and
/// apple plist.
mod config;
mod error;
mod file_util;
mod launchctl;
pub mod server;
