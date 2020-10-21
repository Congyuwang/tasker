/// config module provides general configuration for tasks
pub mod config {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::string::ToString;

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
        label: String,
        #[serde(rename = "Program")]
        program: String,
        #[serde(rename = "Configuration")]
        configuration: Vec<Config>,
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
            self.configuration = self.configuration.into_iter()
                .filter(|c| {&(*c.to_string()) != config_name})
                .collect();
            self
        }

        pub fn from_yaml(yaml: &str) -> serde_yaml::Result<Configuration> {
            serde_yaml::from_str::<Configuration>(yaml)
        }

        pub fn to_yaml(&self) -> serde_yaml::Result<String> {
            serde_yaml::to_string(self)
        }
    }

    #[derive(Deserialize, Serialize, PartialEq, Debug, Display)]
    pub enum Config {
        ProgramArguments(Vec<String>),
        EnvironmentVariables(BTreeMap<String, String>),
        KeepAlive(AliveCondition),
        RunAtLoad(bool),
        WorkingDirectory(String),
        ExitTimeOut(i32),
        StartInterval(i32),
        StartCalendarInterval(Vec<CalendarInterval>),
        StandardInPath(String),
        StandardOutPath(String),
        StandardErrorPath(String),
        SoftResourceLimit(ResourceLimit),
        HardResourceLimits(ResourceLimit)
    }

    impl Config {
        pub fn from_yaml(yaml: &str) -> serde_yaml::Result<Config> {
            serde_yaml::from_str::<Config>(yaml)
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
        minute: Option<i32>,
        #[serde(rename = "Hour")]
        #[serde(skip_serializing_if = "Option::is_none")]
        hour: Option<i32>,
        #[serde(rename = "Day")]
        #[serde(skip_serializing_if = "Option::is_none")]
        day: Option<i32>,
        #[serde(rename = "Weekday")]
        #[serde(skip_serializing_if = "Option::is_none")]
        weekday: Option<i32>,
        #[serde(rename = "Month")]
        #[serde(skip_serializing_if = "Option::is_none")]
        month: Option<i32>,
    }

    /// Resource Limit
    /// <ul>
    ///
    /// <li>Core (integer): <br>
    /// The largest size (in bytes) core file that may be created.</li>
    ///
    /// <li>CPU (integer): <br>
    /// The maximum amount of cpu time (in seconds) to be used by each process.</li>
    ///
    /// <li>Data (integer): <br>
    /// The maximum size (in bytes) of the data segment for a process; this defines how far a program may
    /// extend its break with the sbrk(2) system call.</li>
    ///
    /// <li>FileSize (integer): <br>
    /// The largest size (in bytes) file that may be created.</li>
    ///
    /// <li>MemoryLock (integer): <br>
    /// The maximum size (in bytes) which a process may lock into memory using the mlock(2) function.</li>
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
        #[serde(rename = "Core")]
        #[serde(skip_serializing_if = "Option::is_none")]
        core: Option<i32>,
        #[serde(rename = "CPU")]
        #[serde(skip_serializing_if = "Option::is_none")]
        cpu: Option<i32>,
        #[serde(rename = "Data")]
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<i32>,
        #[serde(rename = "FileSize")]
        #[serde(skip_serializing_if = "Option::is_none")]
        file_size: Option<i32>,
        #[serde(rename = "MemoryLock")]
        #[serde(skip_serializing_if = "Option::is_none")]
        memory_lock: Option<i32>,
        #[serde(rename = "NumberOfFiles")]
        #[serde(skip_serializing_if = "Option::is_none")]
        number_of_files: Option<i32>,
        #[serde(rename = "NumberOfProcesses")]
        #[serde(skip_serializing_if = "Option::is_none")]
        number_of_processes: Option<i32>,
        #[serde(rename = "ResidentSetSize")]
        #[serde(skip_serializing_if = "Option::is_none")]
        resident_set_size: Option<i32>,
        #[serde(rename = "Stack")]
        #[serde(skip_serializing_if = "Option::is_none")]
        stack: Option<i32>,
    }

    #[cfg(test)]
    mod test_config {
        use super::*;

        #[test]
        fn mock_config_yaml() {
            let test_config = Configuration::new("com.tasker.test_task", "/bin/python")
                .add_config(Config::StandardOutPath(
                    "standard_in".parse().unwrap(),
                ))
                .add_config(Config::HardResourceLimits(ResourceLimit {
                    core: None,
                    cpu: None,
                    data: None,
                    file_size: None,
                    memory_lock: None,
                    number_of_files: Some(10000),
                    number_of_processes: Some(8),
                    resident_set_size: None,
                    stack: None
                }))
                .add_config(Config::KeepAlive(
                    AliveCondition {
                        crashed: Some(true),
                        other_job_enabled: Some({
                            let mut other_jobs = BTreeMap::new();
                            other_jobs.insert(String::from("com.tasker.conflict"), false);
                            other_jobs.insert(String::from("com.tasker.depended"), true);
                            other_jobs
                        }),
                        successful_exit: Some(false),
                    }
                ))
                .add_config(Config::StartCalendarInterval(vec![
                    CalendarInterval {
                        minute: Some(15),
                        hour: Some(9),
                        day: None,
                        weekday: None,
                        month: None
                    },
                    CalendarInterval {
                        minute: Some(0),
                        hour: Some(13),
                        day: None,
                        weekday: None,
                        month: None
                    }
                ]))
                .add_config(Config::ProgramArguments(vec![
                    String::from("test_script.py"),
                    String::from("--token=12345678")
                ]))
                .add_config(Config::EnvironmentVariables({
                    let mut env = BTreeMap::new();
                    env.insert(String::from("TOKEN"), String::from("12345678"));
                    env.insert(String::from("ALPHA"), String::from("2.37"));
                    env
                }));

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in\n"
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

            assert_eq!(
                test_config.to_yaml().unwrap(),
                expected_deserialized
            );

            assert_eq!(
                Configuration::from_yaml(&expected_deserialized).unwrap(),
                test_config,
            );
        }

        #[test]
        fn update_test_config() {
            let test_config = Configuration::new("com.tasker.test_task", "/bin/python")
                .add_config(Config::StandardOutPath(
                    "standard_in".parse().unwrap(),
                ))
                .add_config(Config::StandardOutPath(
                    "standard_in_new".parse().unwrap(),
                ));

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in_new";

            assert_eq!(
                test_config.to_yaml().unwrap(),
                expected_deserialized
            );

            assert_eq!(
                Configuration::from_yaml(&expected_deserialized).unwrap(),
                test_config,
            );
        }

        #[test]
        fn update_test_config_from_yaml() {
            let mut test_config = Configuration::new("com.tasker.test_task", "/bin/python")
                .add_config(Config::StandardOutPath(
                    "standard_in".parse().unwrap(),
                ));

            let yaml_to_add = String::new()
                + "---\n"
                + "StartCalendarInterval:\n"
                + "  - Minute: 15\n"
                + "    Hour: 9\n";

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in\n"
                + "  - StartCalendarInterval:\n"
                + "      - Minute: 15\n"
                + "        Hour: 9";

            test_config = test_config.add_config(Config::from_yaml(&yaml_to_add).unwrap());

            assert_eq!(
                test_config.to_yaml().unwrap(),
                expected_deserialized
            );
        }

        #[test]
        fn test_remove_config() {
            let test_config = Configuration::new("com.tasker.test_task", "/bin/python")
                .add_config(Config::StandardOutPath(
                    "standard_in".parse().unwrap(),
                ))
                .add_config(Config::KeepAlive(
                    AliveCondition {
                        crashed: Some(true),
                        successful_exit: Some(false),
                        other_job_enabled: None
                    }
                ))
                .add_config(Config::StartCalendarInterval(vec![
                    CalendarInterval {
                        minute: Some(15),
                        hour: Some(9),
                        day: None,
                        weekday: None,
                        month: None
                    }
                ]))
                .remove_config("KeepAlive");

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in\n"
                + "  - StartCalendarInterval:\n"
                + "      - Minute: 15\n"
                + "        Hour: 9";

            assert_eq!(
                test_config.to_yaml().unwrap(),
                expected_deserialized
            );
        }
    }
}

pub mod launchd {

    mod plist {

        use crate::launchers::config;
        use std::io::{Error, ErrorKind, BufWriter, IntoInnerError, Write};
        use serde::Serialize;
        use std::string::FromUtf8Error;

        fn serde_plist<T>(ser: &T) -> Result<String, FromUtf8Error> where T: Serialize {
            let mut buf = Vec::new();
            plist::to_writer_xml(&mut buf, ser);
            String::from_utf8(buf)
        }

        pub fn get_plist_from_conf(conf: &config::Configuration) -> String {
            let raw_plist = serde_plist(conf).unwrap();
            raw_plist.split('\n')
                .filter(|&line| !line.starts_with("\t\t<dict>")
                    && !line.starts_with("\t\t</dict>")
                    && !line.starts_with("\t<key>Configuration</key>")
                    && !line.starts_with("\t<array>")
                    && !line.starts_with("\t</array>"))
                .map(|line| {line.replacen("\t\t", "", 1)})
                .collect::<Vec<String>>().join("\n")
        }

        #[cfg(test)]
        mod plist_tests {
            use super::*;

            #[test]
            fn test_get_plist() {
                let yaml_config = String::new()
                    + "---\n"
                    + "Label: com.tasker.test_task\n"
                    + "Program: /bin/python\n"
                    + "Configuration:\n"
                    + "  - StandardOutPath: standard_in\n"
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
                    + "\t<string>com.tasker.test_task</string>\n"
                    + "\t<key>Program</key>\n"
                    + "\t<string>/bin/python</string>\n"
                    + "\t<key>StandardOutPath</key>\n"
                    + "\t<string>standard_in</string>\n"
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

                let config = config::Configuration::from_yaml(&yaml_config).unwrap();

                let plist = get_plist_from_conf(&config);

                assert_eq!(plist, expected_plist);

            }
        }
    }

}
