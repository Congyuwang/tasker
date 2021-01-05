use crate::config::Config::ProgramArguments;
use crate::config::{Config, Configuration};
use crate::error::Error;
use crate::initialize::get_environment;
use crate::utils::{
    chown_by_name, create_dir_check, decompress, delete_file_check, execute_command,
    move_by_rename, read_utf8_file,
};
use crate::{
    PLIST_FOLDER, STD_ERR_FILE, STD_OUT_FILE, TASKER_TASK_NAME, TASK_ROOT_ALIAS, TEMP_UNZIP_FOLDER,
};
use regex::Regex;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
pub enum Status {
    RUNNING,
    LOADED,
    UNLOADED,
    NORMAL,
    ERROR,
}

#[derive(Debug, Serialize)]
pub struct TaskInfo {
    pid: Option<i32>,
    last_exit_status: Option<i32>,
    label: String,
    status: Status,
}

fn get_plist_path(label_name: &str) -> PathBuf {
    Path::new(PLIST_FOLDER).join(String::from(label_name) + ".plist")
}

fn get_task_folder_name(label_name: &str) -> PathBuf {
    get_environment().unwrap().task_dir.join(label_name)
}

fn get_output_folder_name(label_name: &str) -> PathBuf {
    get_environment().unwrap().out_dir.join(label_name)
}

fn get_trash_folder_name(label_name: &str) -> PathBuf {
    get_environment().unwrap().trash_dir.join(label_name)
}

///
/// execute launchctl load command, return error if already loaded
///
pub fn load_task(task_label: &str) -> Result<String, Error> {
    if is_loaded(task_label)? {
        return Err(Error::FailedToLoadTask(
            "task is already loaded".to_string(),
        ));
    }
    if !exist(task_label)? {
        return Err(Error::TaskDoesNotExist("no such task to load".to_string()));
    }
    execute_command(Command::new("launchctl").args(&[
        "load",
        get_plist_path(task_label).to_str().unwrap_or_default(),
    ]))
}

///
/// execute launchctl unload command, return error if already unloaded
///
pub fn unload_task(task_label: &str) -> Result<String, Error> {
    if !is_loaded(task_label)? {
        return Err(Error::FailedToUnloadTask(
            "task is already unloaded or does not exist".to_string(),
        ));
    }
    execute_command(Command::new("launchctl").args(&[
        "unload",
        get_plist_path(task_label).to_str().unwrap_or_default(),
    ]))
}

///
/// ignore most failure in this function so as not to be interrupted
/// during deletion.
///
pub fn delete_task(task_label: &str) -> Result<(), Error> {
    match unload_task(task_label) {
        Ok(_) => {}
        Err(_) => {}
    };
    match delete_file_check(get_plist_path(task_label)) {
        Ok(_) => {}
        Err(_) => {}
    };

    // move 'task' folder to trash
    match move_by_rename(
        get_task_folder_name(task_label).as_path(),
        get_trash_folder_name(task_label).as_path(),
    ) {
        Ok(_) => {}
        Err(_) => {}
    };

    // move yaml to trash
    let yaml_in_meta = get_environment()
        .unwrap()
        .meta_dir
        .join(String::from(task_label) + ".yaml");
    if let Some(file_name) = yaml_in_meta.file_name() {
        match std::fs::copy(
            &yaml_in_meta,
            get_trash_folder_name(task_label).join(file_name),
        ) {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    // remove yaml in meta
    match std::fs::remove_file(&yaml_in_meta) {
        Ok(_) => {}
        Err(_) => {}
    };

    // move 'out' folder to trash
    try_clear_output(task_label);
    Ok(())
}

fn try_clear_output(task_label: &str) {
    match move_by_rename(
        get_output_folder_name(task_label).as_path(),
        get_trash_folder_name(task_label).join("out").as_path(),
    ) {
        Ok(_) => {}
        Err(_) => {}
    };
}

///
/// create a new task based on a zip package
///
pub fn create_task(task_zip: &Path) -> Result<(), Error> {
    let unzip_folder = Path::new(TEMP_UNZIP_FOLDER);
    if unzip_folder.metadata().is_ok() {
        match std::fs::remove_dir_all(unzip_folder) {
            Ok(_) => {}
            Err(_) => {
                return Err(Error::FailedToClearTempFolder(
                    "cannot clear folder: ".to_string() + TEMP_UNZIP_FOLDER,
                ))
            }
        }
    }
    decompress(&task_zip, Path::new(TEMP_UNZIP_FOLDER))?;
    let yaml = find_yaml_file(&unzip_folder)?;

    return if let Ok(yaml_content) = read_utf8_file(&yaml) {
        let mut config = Configuration::from_yaml(&yaml_content)?;
        let label = &config.label.clone();

        // process configuration: view `process_config` documentation for detail
        try_clear_output(&label[..]);
        config = process_config(config)?;

        // move yaml to meta folder
        move_yaml_to_meta(&yaml, label)?;

        // move the files to task folder
        let task_folder_name = get_task_folder_name(label);
        create_dir_check(&task_folder_name)?;
        move_by_rename(&unzip_folder, task_folder_name.as_path())?;

        // place plist and load task
        place_plist_and_load(&config, label)
    } else {
        Err(Error::YamlError(
            "error reading yaml as utf8 text".to_string(),
        ))
    };
}

///
/// find the position of yaml in zip package
///
fn find_yaml_file(unzipped_folder: &Path) -> Result<PathBuf, Error> {
    return if let Ok(path) = unzipped_folder.read_dir() {
        // loop over entries
        for entry in path {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext.eq("yaml") {
                        return Ok(path);
                    }
                }
            }
        }
        Err(Error::YamlNotFound("yaml not found".to_owned()))
    } else {
        Err(Error::YamlNotFound(
            "cannot read unzipped folder".to_owned(),
        ))
    };
}

///
/// update yaml after editing yaml
///
pub fn update_yaml(yaml_content: &str, this_label: &str) -> Result<(), Error> {
    let mut config = Configuration::from_yaml(&yaml_content)?;
    let label = &config.label.clone();

    if !label.eq(this_label) {
        return Err(Error::WrongLabelInYaml(format!(
            "label `{}` must be `{}`",
            label, this_label
        )));
    }

    if !exist(label)? {
        return Err(Error::TaskDoesNotExist(format!(
            "task with label `{}` does not exist",
            label
        )));
    }

    if is_loaded(label)? {
        unload_task(label)?;
    }

    try_clear_output(label);

    // process configuration: view `process_config` documentation for detail
    config = process_config(config)?;

    // move yaml in meta folder
    update_yaml_in_meta(yaml_content, label)?;

    // place plist and load task
    place_plist_and_load(&config, label)
}

///
/// this function replaces ROOT_ALIAS to root folder for each task
///
fn replace_task_root_alias(config: &mut Configuration, task_label: &str) -> Result<(), Error> {
    let configuration = &mut config.configuration;
    let task_folder = get_task_folder_name(task_label);
    for conf in configuration {
        if let ProgramArguments(arguments) = conf {
            for arg in arguments {
                if arg.starts_with(TASK_ROOT_ALIAS) {
                    let path_removed_alias = arg.replacen(TASK_ROOT_ALIAS, "", 1);
                    let alias_replaced = task_folder.join(path_removed_alias);
                    if let Some(new_arg) = alias_replaced.to_str() {
                        *arg = new_arg.to_string();
                    } else {
                        return Err(Error::FailedToReplaceRootAlias(
                            "failed to replace root folder, do not use non-utf-8 character in path"
                                .to_string(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

///
/// configuration is processed here:
/// - replace root alias
/// - create output folder if not created
/// - add or override stdout stderr path
///
fn process_config(mut config: Configuration) -> Result<Configuration, Error> {
    let label = &config.label.clone();

    // replace root alias
    replace_task_root_alias(&mut config, label)?;

    // attempt to create task and output folder
    let task_output_name = get_output_folder_name(label);
    create_dir_check(&task_output_name)?;

    // chown for out directory
    chown_by_name(
        &task_output_name,
        config.get_user_name(),
        config.get_group_name(),
    )?;

    let mut temp;

    // add or override stdout stderr path
    if let Some(std_out_file) = task_output_name.join(STD_OUT_FILE).to_str() {
        temp = config.add_config(Config::StandardOutPath(std_out_file.to_string()));
    } else {
        return Err(Error::NonUtfError(
            "non-utf8 character not supported in stdout/stderr path".to_string(),
        ));
    }
    if let Some(std_err_file) = task_output_name.join(STD_ERR_FILE).to_str() {
        temp = temp.add_config(Config::StandardErrorPath(std_err_file.to_string()));
    } else {
        return Err(Error::NonUtfError(
            "non-utf8 character not supported in stdout/stderr path".to_string(),
        ));
    }
    Ok(temp)
}

///
/// move yaml file to meta folder
///
fn move_yaml_to_meta(yaml: &PathBuf, label: &String) -> Result<(), Error> {
    match std::fs::copy(
        &yaml,
        get_environment()
            .unwrap()
            .meta_dir
            .join(String::from(label) + ".yaml")
            .as_path(),
    ) {
        Ok(_) => {}
        Err(_) => {
            return Err(Error::ErrorMoveYamlToMeta(
                "cannot copy yaml to meta folder".to_string(),
            ))
        }
    }

    match std::fs::remove_file(&yaml) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::ErrorMoveYamlToMeta(
            "cannot delete old yaml".to_string(),
        )),
    }
}

fn update_yaml_in_meta(yaml_content: &str, label: &String) -> Result<(), Error> {
    match std::fs::write(
        get_environment()
            .unwrap()
            .meta_dir
            .join(String::from(label) + ".yaml")
            .as_path(),
        yaml_content,
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::FailedToUpdateMetaYaml(
            "cannot write yaml".to_string(),
        )),
    }
}

///
/// put plist into `/Library/LaunchDaemon` and load task
///
fn place_plist_and_load(config: &Configuration, label: &String) -> Result<(), Error> {
    let plist = config.to_plist();
    if let Ok(mut plist_file) = std::fs::File::create(get_plist_path(label)) {
        match plist_file.write_all(plist.as_ref()) {
            Ok(_) => {
                if is_loaded(label)? {
                    unload_task(label)?;
                }
                load_task(label)?;
                Ok(())
            }
            Err(_) => Err(Error::ErrorCreatingPlist("error writing plist".to_string())),
        }
    } else {
        Err(Error::ErrorCreatingPlist("cannot create plist".to_string()))
    }
}

fn is_loaded(label_pattern: &str) -> Result<bool, Error> {
    let task_list = launchctl_list(label_pattern)?;
    for t in task_list {
        if t.label.eq(label_pattern) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn exist(label_pattern: &str) -> Result<bool, Error> {
    let task_list = list_combined(label_pattern)?;
    for t in task_list {
        if t.label.eq(label_pattern) {
            return Ok(true);
        }
    }
    Ok(false)
}

///
/// This function provides an API by returning a JSON of `TaskInfo` returned by
/// `list_combined`
///
pub fn list(label_pattern: &str) -> Result<String, Error> {
    let task_info = list_combined(label_pattern)?;
    match serde_json::to_string_pretty(&task_info) {
        Ok(s) => Ok(s),
        Err(_) => {
            return Err(Error::LaunchctlListError(
                "list error: serialize error".parse().unwrap(),
            ))
        }
    }
}

///
/// This function combines the result of `launchctl_list` and `library_daemons_list`
///
fn list_combined(label_pattern: &str) -> Result<Vec<TaskInfo>, Error> {
    let mut launchctl_info = launchctl_list(label_pattern)?;
    let library_daemons_info = library_daemons_list(label_pattern)?;
    for task in library_daemons_info {
        if !launchctl_info.contains(&task) {
            launchctl_info.insert(task);
        }
    }
    let mut task_info = Vec::new();
    for task in launchctl_info {
        task_info.push(task);
    }
    Ok(task_info)
}

///
/// This function obtains a list of tasks from the launchctl command and
/// convert it into a Set of `TaskInfo`.
///
fn launchctl_list(label_pattern: &str) -> Result<BTreeSet<TaskInfo>, Error> {
    match execute_command(Command::new("launchctl").arg("list")) {
        Ok(list_output) => {
            let task_info = TaskInfo::from_str_filter(&list_output, label_pattern);
            Ok(task_info)
        }
        Err(e) => {
            return Err(Error::LaunchctlListError(format!(
                "failed to list file: {:?}",
                e
            )));
        }
    }
}

///
/// This function obtains a list of tasks from `/Library/LaunchDaemons` folder and
/// convert it into a vector of `TaskInfo`
///
fn library_daemons_list(label_pattern: &str) -> Result<Vec<TaskInfo>, Error> {
    lazy_static! {
        static ref LABEL_REGEX: Regex = Regex::new("^(.+)\\.plist$").unwrap();
    }
    if let Ok(dir) = Path::new(PLIST_FOLDER).read_dir() {
        let mut tasks: Vec<TaskInfo> = Vec::new();
        for file in dir {
            if let Ok(f) = file {
                let path = f.path();
                if let Some(file_name) = f.file_name().to_str() {
                    if path.is_file()
                        && path.extension().unwrap_or_default().eq("plist")
                        && file_name.contains(label_pattern)
                        && file_name.contains(TASKER_TASK_NAME)
                    {
                        if let Some(cap) = LABEL_REGEX.captures(file_name) {
                            if cap.len() == 2 {
                                tasks.push(TaskInfo::from_just_label(&cap[1]));
                            } else {
                                return Err(Error::FailedToReadPlistFolder(String::from(
                                    "fail to find label in plist file name",
                                )));
                            }
                        } else {
                            return Err(Error::FailedToReadPlistFolder(String::from(
                                "fail to capture label in plist file name",
                            )));
                        }
                    }
                } else {
                    return Err(Error::FailedToReadPlistFolder(String::from(
                        "unsupported character in file name",
                    )));
                }
            } else {
                return Err(Error::FailedToReadPlistFolder(
                    String::from("cannot get file info in: ") + &PLIST_FOLDER,
                ));
            }
        }
        Ok(tasks)
    } else {
        Err(Error::FailedToReadPlistFolder(
            String::from("cannot list file in: ") + &PLIST_FOLDER,
        ))
    }
}

pub fn view_yaml(label: &str) -> Result<String, Error> {
    if !exist(label)? {
        return Err(Error::TaskDoesNotExist(
            "attempting to view yaml of non-existent tasks".to_string(),
        ));
    }
    let yaml_file = get_environment()
        .unwrap()
        .meta_dir
        .join(String::from(label) + ".yaml");
    match read_utf8_file(yaml_file.as_path()) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::NonUtfError(format!(
            "cannot find or read yaml file: {:?}",
            e
        ))),
    }
}

pub fn view_std_err(label: &str) -> Result<String, Error> {
    let std_err_file = get_output_folder_name(label).join(STD_ERR_FILE);
    match read_utf8_file(std_err_file.as_path()) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::NonUtfError(format!(
            "task `{}` has not been created or its stderr has not been created: {:?}",
            label, e
        ))),
    }
}

pub fn view_std_out(label: &str) -> Result<String, Error> {
    let std_out_file = get_output_folder_name(label).join(STD_OUT_FILE);
    match read_utf8_file(std_out_file.as_path()) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::NonUtfError(format!(
            "task `{}` has not been created or its stdout has not been created: {:?}",
            label, e
        ))),
    }
}

impl PartialEq for TaskInfo {
    fn eq(&self, other: &Self) -> bool {
        self.label.eq(&other.label)
    }
}

impl Eq for TaskInfo {}

impl PartialOrd for TaskInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.label.partial_cmp(&other.label)
    }
}

impl Ord for TaskInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.label.cmp(&other.label)
    }
}

impl TaskInfo {
    fn from_line(line: &str) -> TaskInfo {
        let mut split = line.split_whitespace();
        let pid: Option<i32> = match split.next().unwrap_or("-").parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => None,
        };
        let last_exit_status = match split.next().unwrap_or("0").parse::<i32>() {
            Ok(d) => Some(d),
            Err(_) => None,
        };
        let label = String::from(split.next().unwrap_or(""));
        let mut status = Status::NORMAL;
        if pid.is_some() {
            status = Status::RUNNING
        } else if last_exit_status.unwrap() != 0 {
            status = Status::ERROR
        } else if view_std_out(&label).is_err() {
            status = Status::LOADED
        }
        TaskInfo {
            pid,
            last_exit_status,
            label,
            status,
        }
    }

    fn from_str_filter(output: &str, pattern: &str) -> BTreeSet<TaskInfo> {
        let mut lines = output.lines();
        let mut temp = Vec::new();
        let mut collected = BTreeSet::new();
        lines.next();
        for line in lines {
            temp.push(TaskInfo::from_line(line))
        }
        for task in temp {
            if task.label.contains(pattern) && task.label.contains(TASKER_TASK_NAME) {
                collected.insert(task);
            }
        }
        collected
    }

    fn from_just_label(label: &str) -> TaskInfo {
        TaskInfo {
            pid: None,
            last_exit_status: None,
            label: label.to_string(),
            status: Status::UNLOADED,
        }
    }
}
