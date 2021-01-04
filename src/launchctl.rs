use crate::config::Config::ProgramArguments;
use crate::config::{Config, Configuration};
use crate::error::Error;
use crate::initialize::get_environment;
use crate::utils::{
    create_dir_check, decompress, delete_file_check, execute_command, move_by_rename,
    read_utf8_file,
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

pub fn load_task(task_label: &str) -> Result<String, Error> {
    if is_loaded(task_label)? {
        return Err(Error::FailedToLoadTask(
            "task is already loaded".to_string(),
        ));
    }
    execute_command(Command::new("launchctl").args(&[
        "load",
        get_plist_path(task_label).to_str().unwrap_or_default(),
    ]))
}

pub fn unload_task(task_label: &str) -> Result<String, Error> {
    if !is_loaded(task_label)? {
        return Err(Error::FailedToUnloadTask(
            "task is already unloaded".to_string(),
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
    // move 'out' folder to trash
    match move_by_rename(
        get_output_folder_name(task_label).as_path(),
        get_trash_folder_name(task_label).join("out").as_path(),
    ) {
        Ok(_) => {}
        Err(_) => {}
    };
    // remove yaml in meta
    match std::fs::remove_file(
        get_environment()
            .unwrap()
            .meta_dir
            .join(String::from(task_label) + ".yaml")
            .as_path(),
    ) {
        Ok(_) => {}
        Err(_) => {}
    };
    Ok(())
}

pub fn replace_task_root_alias(config: &mut Configuration, task_label: &str) -> Result<(), Error> {
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

pub fn create_task(task_zip: &Path) -> Result<(), Error> {
    let unzip_folder = Path::new(TEMP_UNZIP_FOLDER);
    match std::fs::remove_dir_all(unzip_folder) {
        Ok(_) => {}
        Err(_) => {
            return Err(Error::FailedToClearTempFolder(
                "cannot clear folder: ".to_string() + TEMP_UNZIP_FOLDER,
            ))
        }
    };
    decompress(&task_zip, Path::new(TEMP_UNZIP_FOLDER))?;
    let yaml = get_yaml(&unzip_folder)?;

    return if let Ok(yaml_content) = read_utf8_file(&yaml) {
        let mut config = Configuration::from_yaml(&yaml_content)?;
        let label = &config.label.clone();

        if is_loaded(label)? {
            unload_task(label)?;
        }

        replace_task_root_alias(&mut config, label)?;

        // attempt to create task and output folder
        let task_folder_name = get_task_folder_name(label);
        let task_output_name = get_output_folder_name(label);
        create_dir_check(&task_folder_name)?;
        create_dir_check(&task_output_name)?;

        // create stdout and stderr files
        if let Some(std_out_file) = task_output_name.join(STD_OUT_FILE).to_str() {
            config = config.add_config(Config::StandardOutPath(std_out_file.to_string()));
        } else {
            return Err(Error::NonUtfError(
                "non-utf8 character not supported in stdout/stderr path".to_string(),
            ));
        }
        if let Some(std_err_file) = task_output_name.join(STD_ERR_FILE).to_str() {
            config = config.add_config(Config::StandardErrorPath(std_err_file.to_string()));
        } else {
            return Err(Error::NonUtfError(
                "non-utf8 character not supported in stdout/stderr path".to_string(),
            ));
        }

        // copy yaml to meta folder
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
                return Err(Error::ErrorCopyYamlToMeta(
                    "error writing plist".to_string(),
                ))
            }
        }

        // move the files to task folder
        move_by_rename(&unzip_folder, task_folder_name.as_path())?;

        // place plist
        let plist = config.to_plist();
        if let Ok(mut plist_file) = std::fs::File::create(get_plist_path(label)) {
            match plist_file.write_all(plist.as_ref()) {
                Ok(_) => {
                    load_task(label)?;
                    Ok(())
                }
                Err(_) => Err(Error::ErrorCreatingPlist("error writing plist".to_string())),
            }
        } else {
            Err(Error::ErrorCreatingPlist("cannot create plist".to_string()))
        }
    } else {
        Err(Error::YamlError(
            "error reading yaml as utf8 text".to_string(),
        ))
    };
}

pub fn get_yaml(unzipped_folder: &Path) -> Result<PathBuf, Error> {
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

pub fn is_loaded(label_pattern: &str) -> Result<bool, Error> {
    let task_list = launchctl_list(label_pattern)?;
    for t in task_list {
        if t.label.eq(label_pattern) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn list(label_pattern: &str) -> Result<String, Error> {
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
    match serde_json::to_string_pretty(&task_info) {
        Ok(s) => Ok(s),
        Err(_) => {
            return Err(Error::LaunchctlListError(
                "list error: serialize error".parse().unwrap(),
            ))
        }
    }
}

pub fn launchctl_list(label_pattern: &str) -> Result<BTreeSet<TaskInfo>, Error> {
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

pub fn library_daemons_list(label_pattern: &str) -> Result<Vec<TaskInfo>, Error> {
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

pub fn view_std_err(label: &str) -> Result<String, Error> {
    let std_err_file = get_output_folder_name(label).join(STD_ERR_FILE);
    match read_utf8_file(std_err_file.as_path()) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::NonUtfError(format!(
            "cannot find or read file: {:?}",
            e
        ))),
    }
}

pub fn view_std_out(label: &str) -> Result<String, Error> {
    let std_out_file = get_output_folder_name(label).join(STD_OUT_FILE);
    match read_utf8_file(std_out_file.as_path()) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::NonUtfError(format!(
            "cannot find or read file: {:?}",
            e
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
