use std::process::{Command, Output};

#[derive(Debug)]
pub struct TaskInfo {
    pid: Option<i32>,
    status: i32,
    label: String
}

fn list() -> std::io::Result<Output> {
    Command::new("launchctl")
        .arg("list")
        .output()
}

impl TaskInfo {

    fn from_line(line: &str) -> TaskInfo {
        let mut split = line.split_whitespace();
        let pid: Option<i32> = match split.next().unwrap_or("-").parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => None
        };
        let status: i32 = split.next().unwrap_or("0").parse::<i32>().unwrap_or(0);
        let label = String::from(split.next().unwrap_or(""));
        TaskInfo {
            pid,
            status,
            label
        }
    }

    pub fn from_str_filter(output: &str, pattern: &str) -> Vec<TaskInfo> {
        let mut lines = output.lines();
        let mut returned = Vec::new();
        lines.next();
        for line in lines {
            if line.contains(pattern) {
                returned.push(TaskInfo::from_line(line))
            }
        }
        returned
    }

    pub fn from_str(output: &str) -> Vec<TaskInfo> {
        TaskInfo::from_str_filter(output, "")
    }

}

#[cfg(test)]
mod test_launchctl {
    use super::*;

    #[test]
    fn test_list() {
        let output = list().expect("failed to execute");
        println!("status: {}", &output.status);
        let stdout = std::str::from_utf8(&output.stdout).expect("stdout not utf8");
        println!("stderr: {}", std::str::from_utf8(&output.stderr).expect("stderr not utf8"));
        for task_info in TaskInfo::from_str(stdout) {
            println!("{:?}", task_info);
        }
        println!("----");
        for task_info in TaskInfo::from_str_filter(stdout, "com.apple.mdworker") {
            println!("{:?}", task_info);
        }
    }
}
