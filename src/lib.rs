#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum_macros;

static TASKER_TASK_NAME: &str = "com.tasker.tasks";
static PLIST_FOLDER: &str = "/Library/LaunchDaemons/";
static TEMP_UNZIP_FOLDER: &str = "/tmp/tasker.task.com/temp_unzip/";
static TEMP_ZIP_FOLDER: &str = "/tmp/tasker.task.com/temp_zip/";
static TEMP_ZIP_PATH: &str = "/tmp/tasker.task.com/";
static TASK_ROOT_ALIAS: &str = "~root~/";
static STD_OUT_FILE: &str = "stdout.log";
static STD_ERR_FILE: &str = "stderr.log";
static MAX_OUTPUT_LINE: usize = 5000;

/// the config module provides api to convert task configuration to and from yaml and
/// apple plist.
mod config;
mod error;
pub mod initialize;
mod launchctl;
pub mod server;
mod utils;
