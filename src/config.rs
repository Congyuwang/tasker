use crate::error::Error;
use crate::TASKER_TASK_NAME;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::string::FromUtf8Error;
use std::string::ToString;

static LABEL_REG: &str = "^[A-Za-z0-9_]+(\\.[A-Za-z0-9_]+)*$";

macro_rules! check_range_return_err {
    ($name: ident, $i: expr, $lo: expr, $hi: expr) => {
        if $i < $lo || $i > $hi {
            return Err(Error::ConfigRangeError(format!(
                "`{}` with value `{:?}` is out of range ({}, {})",
                stringify!($name),
                $i,
                $lo,
                $hi
            )));
        }
    };
}

macro_rules! check_option_range_return_err {
    ($self: ident, $field_name: ident, $lo: expr, $hi: expr) => {
        if let Some(i) = $self.$field_name {
            check_range_return_err!($field_name, i, $lo, $hi);
        }
    };
}

/// Config:
/// This enum is used to be directly mapped to launchd.plist XML.
/// It is not meant to be directly used by library user.<br>
/// <p>
/// Notes:
/// The Program key must be an absolute path.
/// </p>
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Configuration {
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "Program")]
    program: String,
    #[serde(rename = "Configuration")]
    pub configuration: Vec<Config>,
}

impl Configuration {
    fn new(label: &str, program: &str) -> Configuration {
        Configuration {
            label: String::from(label),
            program: String::from(program),
            configuration: Vec::new(),
        }
    }

    /// add_config() function <i>add</i> new configuration or <i>replace</i> old configuration.
    /// This function does not do any checking
    pub fn add_config(mut self, config: Config) -> Configuration {
        let conf_name = config.to_string();
        let configuration = &mut self.configuration;
        for conf in configuration {
            if conf_name == conf.to_string() {
                *conf = config;
                return self;
            }
        }
        self.configuration.push(config);
        self
    }

    pub fn remove_config(mut self, config_name: &str) -> Configuration {
        self.configuration = self
            .configuration
            .into_iter()
            .filter(|c| &(*c.to_string()) != config_name)
            .collect();
        self
    }

    /// this function does checking, and removes duplicates to keep the last items
    pub fn from_yaml(yaml: &str) -> Result<Configuration, Error> {
        let config = match serde_yaml::from_str::<Configuration>(yaml) {
            Ok(config) => config,
            Err(e) => return Err(Error::YamlError(e.to_string())),
        }
        .check_label()?
        .check_program()?
        .append_domain();

        let mut new_config = Configuration::new(&config.label, &config.program);
        for c in config.configuration {
            new_config = new_config.add_config(c.check()?);
        }
        Ok(new_config)
    }

    pub fn to_yaml(&self) -> serde_yaml::Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        let mut result = Vec::new();
        let label_line = String::from("Label: ") + TASKER_TASK_NAME + ".";
        for y in yaml.lines() {
            if y.starts_with(&label_line) {
                result.push(y.replace(&label_line, "Label: "));
            } else {
                result.push(String::from(y))
            }
        }
        Ok(result.join("\n"))
    }

    pub fn to_plist(&self) -> String {
        let raw_plist = Configuration::serde_plist(self).unwrap();
        raw_plist
            .lines()
            .filter(|&line| {
                !line.starts_with("\t\t<dict>")
                    && !line.starts_with("\t\t</dict>")
                    && !line.starts_with("\t<key>Configuration</key>")
                    && !line.starts_with("\t<array>")
                    && !line.starts_with("\t</array>")
            })
            .map(|line| line.replacen("\t\t", "", 1))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn serde_plist<T>(ser: &T) -> Result<String, FromUtf8Error>
    where
        T: Serialize,
    {
        let mut buf = Vec::new();
        plist::to_writer_xml(&mut buf, ser).expect("inner error (function: serde_plist)");
        String::from_utf8(buf)
    }

    fn check_program(self) -> Result<Configuration, Error> {
        let program = Path::new(&self.program);
        if !program.is_absolute() {
            return Err(Error::ConfigProgramError(format!(
                "program path `{}` is not an absolute path",
                &self.program
            )));
        }
        if !program.is_file() {
            return Err(Error::ConfigProgramError(format!(
                "program `{}` is not found or not permitted to access",
                &self.program
            )));
        }
        Ok(self)
    }

    ///
    /// label only allows patterns as follows `[A-Za-z]+(\\.[A-Za-z]+)*`.
    ///
    fn check_label(self) -> Result<Configuration, Error> {
        lazy_static! {
            static ref LABEL_REGEX: Regex = Regex::new(LABEL_REG).unwrap();
        }
        if !LABEL_REGEX.is_match(&self.label) {
            return Err(Error::ConfigLabelError(format!(
                "`{}` is not a valid label",
                &self.label
            )));
        }
        Ok(self)
    }

    fn append_domain(mut self) -> Configuration {
        self.label = String::from(TASKER_TASK_NAME) + "." + &self.label;
        self
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Display)]
pub enum Config {
    ProgramArguments(Vec<String>),
    EnvironmentVariables(BTreeMap<String, String>),
    KeepAlive(AliveCondition),
    RunAtLoad(bool),
    WorkingDirectory(String),
    UserName(String),
    GroupName(String),
    RootDirectory(String),
    ExitTimeOut(i64),
    StartInterval(i64),
    StartCalendarInterval(Vec<CalendarInterval>),
    StandardInPath(String),
    StandardOutPath(String),
    StandardErrorPath(String),
    SoftResourceLimit(ResourceLimit),
    HardResourceLimits(ResourceLimit),
}

impl Config {
    /// this function does checking
    pub fn from_yaml(yaml: &str) -> Result<Config, Error> {
        match serde_yaml::from_str::<Config>(yaml) {
            Ok(config) => config.check(),
            Err(e) => Err(Error::YamlError(e.to_string())),
        }
    }

    fn check(self) -> Result<Config, Error> {
        match self {
            Config::SoftResourceLimit(limit) => match limit.check() {
                Ok(l) => Ok(Config::SoftResourceLimit(l)),
                Err(e) => Err(e),
            },
            Config::HardResourceLimits(limit) => match limit.check() {
                Ok(l) => Ok(Config::HardResourceLimits(l)),
                Err(e) => Err(e),
            },
            Config::StartCalendarInterval(calendar) => {
                let mut new_cals = Vec::with_capacity(calendar.len());
                for cal in calendar {
                    match cal.check() {
                        Ok(c) => new_cals.push(c),
                        Err(e) => return Err(e),
                    }
                }
                Ok(Config::StartCalendarInterval(new_cals))
            }
            Config::ExitTimeOut(t) => {
                check_range_return_err!(ExitTimeOut, t, 0, i64::MAX);
                Ok(Config::ExitTimeOut(t))
            }
            Config::StartInterval(t) => {
                check_range_return_err!(StartInterval, t, 0, i64::MAX);
                Ok(Config::StartInterval(t))
            }
            Config::WorkingDirectory(p) => {
                let p: String = Config::check_path(p)?;
                Ok(Config::WorkingDirectory(p))
            }
            Config::RootDirectory(p) => {
                let p: String = Config::check_path(p)?;
                Ok(Config::RootDirectory(p))
            }
            Config::StandardInPath(p) => {
                let p: String = Config::check_file(p)?;
                Ok(Config::StandardInPath(p))
            }
            Config::StandardOutPath(p) => {
                let p: String = Config::check_file(p)?;
                Ok(Config::StandardOutPath(p))
            }
            Config::StandardErrorPath(p) => {
                let p: String = Config::check_file(p)?;
                Ok(Config::StandardErrorPath(p))
            }
            _ => Ok(self),
        }
    }

    fn check_path(path: String) -> Result<String, Error> {
        if !Path::new(&path).is_dir() {
            return Err(Error::ConfigPathError(format!(
                "`{}` is not a directory",
                path
            )));
        }
        Ok(path)
    }

    fn check_file(path: String) -> Result<String, Error> {
        if !Path::new(&path).is_file() {
            return Err(Error::ConfigPathError(format!("`{}` is not a file", path)));
        }
        Ok(path)
    }
}

/// AliveCondition
///
/// <ul>
///
/// <li>SuccessfulExit (boolean):<br>
/// If true, the job will be restarted as long as the program exits and with an exit status of zero. If
/// false, the job will be restarted in the inverse condition.  This key implies that "RunAtLoad" is set to
/// true, since the job needs to run at least once before an exit status can be determined.</li>
///
/// <li>OtherJobEnabled (dictionary of booleans):<br>
/// Each key in this dictionary is the name of another job. If the value is true, then the job will be kept
/// alive as long as one of the specified other jobs is loaded in launchd(8).</li>
///
/// <p>
/// NOTE: This key only evaluates whether the job is loaded, not whether it is running. Use of this key is
/// highly discouraged. If multiple jobs need to coordinate coordinate their lifecycles, they should
/// establish contracts using IPC.
/// </p>
///
/// <li>Crashed (boolean):<br>
/// If true, the the job will be restarted as long as it exited due to a signal which is typically
/// associated with a crash (SIGILL, SIGSEGV, etc.). If false, the job will be restarted in the
/// inverse condition.</li>
///
/// </ul>
#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct AliveCondition {
    #[serde(rename = "SuccessfulExit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    successful_exit: Option<bool>,
    #[serde(rename = "OtherJobEnabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    other_job_enabled: Option<BTreeMap<String, bool>>,
    #[serde(rename = "Crashed")]
    #[serde(skip_serializing_if = "Option::is_none")]
    crashed: Option<bool>,
}

/// Calendar intervals
/// <ul>
/// <li>Minute (integer):<br>
/// The minute (0-59) on which this job will be run. </li>
/// <li>Hour (integer):<br>
/// The hour (0-23) on which this job will be run. </li>
/// <li>Day (integer):<br>
/// The day of the month (1-31) on which this job will be run. </li>
/// <li>Weekday (integer):<br>
/// The weekday on which this job will be run (0 and 7 are Sunday).
/// If both Day and Weekday are specified, then the job will be started if
/// either one matches the current date. </li>
/// <li>Month (integer):<br>
/// The month (1-12) on which this job will be run.</li>
/// </ul>
#[derive(Deserialize, Serialize, PartialEq, Debug, Hash, Eq)]
pub struct CalendarInterval {
    #[serde(rename = "Minute")]
    #[serde(skip_serializing_if = "Option::is_none")]
    minute: Option<i64>,
    #[serde(rename = "Hour")]
    #[serde(skip_serializing_if = "Option::is_none")]
    hour: Option<i64>,
    #[serde(rename = "Day")]
    #[serde(skip_serializing_if = "Option::is_none")]
    day: Option<i64>,
    #[serde(rename = "Weekday")]
    #[serde(skip_serializing_if = "Option::is_none")]
    weekday: Option<i64>,
    #[serde(rename = "Month")]
    #[serde(skip_serializing_if = "Option::is_none")]
    month: Option<i64>,
}

impl CalendarInterval {
    pub fn check(self) -> Result<CalendarInterval, Error> {
        check_option_range_return_err!(self, minute, 0, 59);
        check_option_range_return_err!(self, hour, 0, 23);
        check_option_range_return_err!(self, day, 1, 31);
        check_option_range_return_err!(self, weekday, 0, 7);
        check_option_range_return_err!(self, month, 1, 12);
        Ok(self)
    }
}

/// Resource Limit
/// <ul>
///
/// <li>FileSize (integer): <br>
/// The largest size (in bytes) file that may be created.</li>
///
/// <li>NumberOfFiles (integer): <br>
/// The maximum number of open files for this process.  Setting this value in a system wide daemon will set
/// the sysctl(3) kern.maxfiles (SoftResourceLimits) or kern.maxfilesperproc (HardResourceLimits) value in
/// addition to the setrlimit(2) values.</li>
///
/// <li>NumberOfProcesses (integer): <br>
/// The maximum number of simultaneous processes for this UID. Setting this value in a system wide daemon
/// will set the sysctl(3) kern.maxproc (SoftResourceLimits) or kern.maxprocperuid (HardResourceLimits)
/// value in addition to the setrlimit(2) values.</li>
///
/// <li>ResidentSetSize (integer): <br>
/// The maximum size (in bytes) to which a process's resident set size may grow.  This imposes a limit on
/// the amount of physical memory to be given to a process; if memory is tight, the system will prefer to
/// take memory from processes that are exceeding their declared resident set size.</li>
///
/// <li>Stack (integer): <br>
/// The maximum size (in bytes) of the stack segment for a process; this defines how far a program's stack
/// segment may be extended.  Stack extension is performed automatically by the system.</li>
///
/// </ul>
#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct ResourceLimit {
    #[serde(rename = "CPU")]
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu: Option<i64>,
    #[serde(rename = "FileSize")]
    #[serde(skip_serializing_if = "Option::is_none")]
    file_size: Option<i64>,
    #[serde(rename = "NumberOfFiles")]
    #[serde(skip_serializing_if = "Option::is_none")]
    number_of_files: Option<i64>,
    #[serde(rename = "NumberOfProcesses")]
    #[serde(skip_serializing_if = "Option::is_none")]
    number_of_processes: Option<i64>,
    #[serde(rename = "ResidentSetSize")]
    #[serde(skip_serializing_if = "Option::is_none")]
    resident_set_size: Option<i64>,
    #[serde(rename = "Stack")]
    #[serde(skip_serializing_if = "Option::is_none")]
    stack: Option<i64>,
}

impl ResourceLimit {
    pub fn check(self) -> Result<ResourceLimit, Error> {
        check_option_range_return_err!(self, file_size, 0, i64::MAX);
        check_option_range_return_err!(self, number_of_files, 0, i64::MAX);
        check_option_range_return_err!(self, number_of_processes, 0, 500);
        check_option_range_return_err!(self, resident_set_size, 0, i64::MAX);
        check_option_range_return_err!(self, stack, 0, 67104768);
        Ok(self)
    }
}

#[cfg(test)]
mod test_config_mod {
    use super::*;

    #[test]
    fn mock_config_yaml() {
        let test_config = Configuration::new("com.tasker.tasks.test_task", "/usr/bin/python")
            .add_config(Config::StandardOutPath("/tmp/".parse().unwrap()))
            .add_config(Config::HardResourceLimits(ResourceLimit {
                cpu: None,
                file_size: None,
                number_of_files: Some(10000),
                number_of_processes: Some(8),
                resident_set_size: None,
                stack: None,
            }))
            .add_config(Config::KeepAlive(AliveCondition {
                crashed: Some(true),
                other_job_enabled: Some({
                    let mut other_jobs = BTreeMap::new();
                    other_jobs.insert(String::from("com.tasker.conflict"), false);
                    other_jobs.insert(String::from("com.tasker.depended"), true);
                    other_jobs
                }),
                successful_exit: Some(false),
            }))
            .add_config(Config::StartCalendarInterval(vec![
                CalendarInterval {
                    minute: Some(15),
                    hour: Some(9),
                    day: None,
                    weekday: None,
                    month: None,
                },
                CalendarInterval {
                    minute: Some(0),
                    hour: Some(13),
                    day: None,
                    weekday: None,
                    month: None,
                },
            ]))
            .add_config(Config::ProgramArguments(vec![
                String::from("test_script.py"),
                String::from("--token=12345678"),
            ]))
            .add_config(Config::EnvironmentVariables({
                let mut env = BTreeMap::new();
                env.insert(String::from("TOKEN"), String::from("12345678"));
                env.insert(String::from("ALPHA"), String::from("2.37"));
                env
            }));

        let expected_deserialized = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - HardResourceLimits:\n"
            + "      NumberOfFiles: 10000\n"
            + "      NumberOfProcesses: 8\n"
            + "  - KeepAlive:\n"
            + "      SuccessfulExit: false\n"
            + "      OtherJobEnabled:\n"
            + "        com.tasker.conflict: false\n"
            + "        com.tasker.depended: true\n"
            + "      Crashed: true\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9\n"
            + "      - Minute: 0\n"
            + "        Hour: 13\n"
            + "  - ProgramArguments:\n"
            + "      - test_script.py\n"
            + "      - \"--token=12345678\"\n"
            + "  - EnvironmentVariables:\n"
            + "      ALPHA: \"2.37\"\n"
            + "      TOKEN: \"12345678\"";

        assert_eq!(test_config.to_yaml().unwrap(), expected_deserialized);

        assert_eq!(
            Configuration::from_yaml(&expected_deserialized).unwrap(),
            test_config,
        );
    }

    #[test]
    fn update_test_config() {
        let test_config = Configuration::new("com.tasker.tasks.test_task", "/usr/bin/python")
            .add_config(Config::StandardOutPath("/tmp/".parse().unwrap()))
            .add_config(Config::StandardOutPath("/var/tmp/".parse().unwrap()));

        let expected_deserialized = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /var/tmp/";

        assert_eq!(test_config.to_yaml().unwrap(), expected_deserialized);

        assert_eq!(
            Configuration::from_yaml(&expected_deserialized).unwrap(),
            test_config,
        );
    }

    #[test]
    fn update_test_config_from_yaml() {
        let mut test_config = Configuration::new("test_task", "/usr/bin/python")
            .add_config(Config::StandardOutPath("/tmp/".parse().unwrap()));

        let yaml_to_add = String::new()
            + "---\n"
            + "StartCalendarInterval:\n"
            + "  - Minute: 15\n"
            + "    Hour: 9\n";

        let expected_deserialized = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9";

        test_config = test_config.add_config(Config::from_yaml(&yaml_to_add).unwrap());

        assert_eq!(test_config.to_yaml().unwrap(), expected_deserialized);
    }

    #[test]
    fn test_remove_config() {
        let test_config = Configuration::new("test_task", "/usr/bin/python")
            .add_config(Config::StandardOutPath("/tmp/".parse().unwrap()))
            .add_config(Config::KeepAlive(AliveCondition {
                crashed: Some(true),
                successful_exit: Some(false),
                other_job_enabled: None,
            }))
            .add_config(Config::StartCalendarInterval(vec![CalendarInterval {
                minute: Some(15),
                hour: Some(9),
                day: None,
                weekday: None,
                month: None,
            }]))
            .remove_config("KeepAlive");

        let expected_deserialized = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9";

        assert_eq!(test_config.to_yaml().unwrap(), expected_deserialized);
    }

    #[test]
    fn test_get_plist() {
        let yaml_config = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - KeepAlive:\n"
            + "      Crashed: true\n"
            + "      OtherJobEnabled:\n"
            + "        com.tasker.conflict: false\n"
            + "        com.tasker.depended: true\n"
            + "      SuccessfulExit: false\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9\n"
            + "      - Minute: 0\n"
            + "        Hour: 13\n"
            + "  - ProgramArguments:\n"
            + "      - test_script.py\n"
            + "      - \"--token=12345678\"\n"
            + "  - EnvironmentVariables:\n"
            + "      TOKEN: 12345678";

        let expected_plist = String::new()
            + "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
            + "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"
            + "<plist version=\"1.0\">\n"
            + "<dict>\n"
            + "\t<key>Label</key>\n"
            + "\t<string>"
            + &TASKER_TASK_NAME
            + "."
            + "test_task</string>\n"
            + "\t<key>Program</key>\n"
            + "\t<string>/usr/bin/python</string>\n"
            + "\t<key>StandardOutPath</key>\n"
            + "\t<string>/tmp/</string>\n"
            + "\t<key>KeepAlive</key>\n"
            + "\t<dict>\n"
            + "\t\t<key>SuccessfulExit</key>\n"
            + "\t\t<false />\n"
            + "\t\t<key>OtherJobEnabled</key>\n"
            + "\t\t<dict>\n"
            + "\t\t\t<key>com.tasker.conflict</key>\n"
            + "\t\t\t<false />\n"
            + "\t\t\t<key>com.tasker.depended</key>\n"
            + "\t\t\t<true />\n"
            + "\t\t</dict>\n"
            + "\t\t<key>Crashed</key>\n"
            + "\t\t<true />\n"
            + "\t</dict>\n"
            + "\t<key>StartCalendarInterval</key>\n"
            + "\t<array>\n"
            + "\t\t<dict>\n"
            + "\t\t\t<key>Minute</key>\n"
            + "\t\t\t<integer>15</integer>\n"
            + "\t\t\t<key>Hour</key>\n"
            + "\t\t\t<integer>9</integer>\n"
            + "\t\t</dict>\n"
            + "\t\t<dict>\n"
            + "\t\t\t<key>Minute</key>\n"
            + "\t\t\t<integer>0</integer>\n"
            + "\t\t\t<key>Hour</key>\n"
            + "\t\t\t<integer>13</integer>\n"
            + "\t\t</dict>\n"
            + "\t</array>\n"
            + "\t<key>ProgramArguments</key>\n"
            + "\t<array>\n"
            + "\t\t<string>test_script.py</string>\n"
            + "\t\t<string>--token=12345678</string>\n"
            + "\t</array>\n"
            + "\t<key>EnvironmentVariables</key>\n"
            + "\t<dict>\n"
            + "\t\t<key>TOKEN</key>\n"
            + "\t\t<string>12345678</string>\n"
            + "\t</dict>\n"
            + "</dict>\n"
            + "</plist>";

        let config = Configuration::from_yaml(&yaml_config).unwrap();

        let plist = config.to_plist();

        assert_eq!(plist, expected_plist);
    }

    #[test]
    #[should_panic]
    fn no_such_attribute() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9\n"
            + "  - NoSuchAttribute";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }

    #[test]
    #[should_panic(expected = "missing field `Program`")]
    fn missing_attribute() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 9\n";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }

    #[test]
    #[should_panic(expected = "`hour` with value `30` is out of range (0, 23)")]
    fn config_range_panic_hour() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 30";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }

    #[test]
    #[should_panic(expected = "program path `python` is not an absolute path")]
    fn config_panic_program_path() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: /tmp/\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 20";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }

    #[test]
    #[should_panic(expected = "program `/python` is not found or not permitted to access")]
    fn config_panic_program_not_found() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: \"/tmp/\"\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 20";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }

    #[test]
    #[should_panic(expected = "`/tmp/no such path` is not a directory")]
    fn config_panic_standard_out_path() {
        let yaml = String::new()
            + "---\n"
            + "Label: test_task\n"
            + "Program: /usr/bin/python\n"
            + "Configuration:\n"
            + "  - StandardOutPath: \"/tmp/no such path\"\n"
            + "  - StartCalendarInterval:\n"
            + "      - Minute: 15\n"
            + "        Hour: 20";

        let config = Configuration::from_yaml(&yaml).unwrap();
    }
}
