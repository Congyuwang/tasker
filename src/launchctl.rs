use crate::config::{Config, Configuration};
use crate::error::Error;
use crate::initialize::get_environment;
use crate::utils::{
    create_dir_check, create_file_check, decompress, delete_file_check, move_by_rename,
    read_utf8_file,
};
use crate::{PLIST_FOLDER, TASKER_TASK_NAME, TEMP_UNZIP_FOLDER};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Serialize)]
pub struct TaskInfo {
    pid: Option<i32>,
    status: i32,
    label: String,
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

pub fn load_task(task_label: &str) -> Result<(), Error> {
    return match Command::new("launchctl")
        .args(&["load", get_plist_path(task_label).to_str().unwrap_or_default()])
        .output() {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::FailedToLoadTask(format!("failed to load {}", task_label)))
    }
}

pub fn unload_task(task_label: &str) -> Result<(), Error> {
    return match Command::new("launchctl")
        .args(&["unload", get_plist_path(task_label).to_str().unwrap_or_default()])
        .output() {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::FailedToUnloadTask(format!("failed to unload {}", task_label)))
    }
}

pub fn delete_task(task_label: &str) -> Result<(), Error> {
    unload_task(task_label)?;
    delete_file_check(get_plist_path(task_label))?;
    move_by_rename(
        get_task_folder_name(task_label).as_path(),
        get_trash_folder_name(task_label).as_path(),
    )?;
    Ok(())
}

pub fn create_task(task_zip: &Path) -> Result<(), Error> {
    let unzip_folder = Path::new(TEMP_UNZIP_FOLDER);
    decompress(&task_zip, Path::new(TEMP_UNZIP_FOLDER))?;
    let yaml = get_yaml(&unzip_folder)?;

    return if let Ok(yaml_content) = read_utf8_file(&yaml) {
        let mut config = Configuration::from_yaml(&yaml_content)?;

        // attempt to create task and output folder
        let task_folder_name = get_task_folder_name(&config.label);
        let task_output_name = get_output_folder_name(&config.label);
        create_dir_check(&task_folder_name)?;
        create_dir_check(&task_output_name)?;

        // create stdout and stderr files
        if let Some(std_out_file) = task_output_name.join("stdout.log").to_str() {
            config = config.add_config(Config::StandardOutPath(std_out_file.to_string()));
        } else {
            return Err(Error::NonUtfError(
                "non-utf8 character not supported in stdout/stderr path".to_string(),
            ));
        }
        if let Some(std_err_file) = task_output_name.join("stderr.log").to_str() {
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
                .join(String::from(&config.label) + ".yaml")
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
        if let Ok(mut plist_file) = std::fs::File::create(get_plist_path(&config.label)) {
            match plist_file.write_all(plist.as_ref()) {
                Ok(_) => {
                    load_task(&config.label)?;
                    Ok(())
                },
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

pub fn list(label_pattern: &str) -> Result<String, Error> {
    let list_output = list_inner();
    let list_output = match list_output {
        Ok(o) => o,
        Err(_) => {
            return Err(Error::LaunchctlListError(
                "failed to list file".parse().unwrap(),
            ))
        }
    };
    if !list_output.status.success() {
        return Err(Error::LaunchctlListError(format!(
            "failed to list file: {}",
            std::str::from_utf8(&list_output.stderr).unwrap()
        )));
    };
    let task_info = TaskInfo::from_str_filter(
        std::str::from_utf8(&list_output.stdout).unwrap(),
        label_pattern,
    );
    match serde_json::to_string_pretty(&task_info) {
        Ok(s) => Ok(s),
        Err(_) => {
            return Err(Error::LaunchctlListError(
                "list error: serialize error".parse().unwrap(),
            ))
        }
    }
}

fn list_inner() -> std::io::Result<Output> {
    Command::new("launchctl").arg("list").output()
}

impl TaskInfo {
    fn from_line(line: &str) -> TaskInfo {
        let mut split = line.split_whitespace();
        let pid: Option<i32> = match split.next().unwrap_or("-").parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => None,
        };
        let status: i32 = split.next().unwrap_or("0").parse::<i32>().unwrap_or(0);
        let label = String::from(split.next().unwrap_or(""));
        TaskInfo { pid, status, label }
    }

    fn from_str_filter(output: &str, pattern: &str) -> Vec<TaskInfo> {
        let mut lines = output.lines();
        let mut temp = Vec::new();
        let mut returned = Vec::new();
        lines.next();
        for line in lines {
            temp.push(TaskInfo::from_line(line))
        }
        for task in temp {
            if task.label.contains(pattern) && task.label.contains(TASKER_TASK_NAME) {
                returned.push(task);
            }
        }
        returned
    }
}

#[cfg(test)]
mod test_launchctl {
    use super::*;

    #[test]
    fn test_list() {
        let result = list("com.apple.mdworker.mail").unwrap();
        let expected = String::new()
            + "[\n"
            + "  {\n"
            + "    \"pid\": null,\n"
            + "    \"status\": 0,\n"
            + "    \"label\": \"com.apple.mdworker.mail\"\n"
            + "  }\n"
            + "]";
        assert_eq!(result, expected);
    }
}
