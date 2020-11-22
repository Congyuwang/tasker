use crate::error::Error;
use serde::Serialize;
use std::process::{Command, Output};

#[derive(Debug, Serialize)]
pub struct TaskInfo {
    pid: Option<i32>,
    status: i32,
    label: String,
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
            if task.label.contains(pattern) {
                returned.push(task);
            }
        }
        returned
    }

    fn from_str(output: &str) -> Vec<TaskInfo> {
        TaskInfo::from_str_filter(output, "")
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
