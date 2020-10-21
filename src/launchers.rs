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
        KeepAlive(Vec<AliveCondition>),
        RunAtLoad(bool),
        WorkingDirectory(String),
        ExitTimeOut(i32),
        StartInterval(i32),
        StartCalendarInterval(Vec<CalendarInterval>),
        StandardInPath(String),
        StandardOutPath(String),
        StandardErrorPath(String),
        SoftResourceLimit(Vec<Limit>),
        HardResourceLimits(Vec<Limit>)
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
    /// <li>Always: always alive.</li>
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
    pub enum AliveCondition {
        Always,
        SuccessfulExit(bool),
        OtherJobEnabled(BTreeMap<String, bool>),
        Crashed(bool),
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
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    pub enum CalendarInterval {
        Minute(i32),
        Hour(i32),
        Day(i32),
        Weekday(i32),
        Month(i32),
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
    pub enum Limit {
        Core(i32),
        CPU(i32),
        Data(i32),
        FileSize(i32),
        MemoryLock(i32),
        NumberOfFiles(i32),
        NumberOfProcesses(i32),
        ResidentSetSize(i32),
        Stack(i32),
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
                .add_config(Config::HardResourceLimits(vec![
                    Limit::NumberOfFiles(10000),
                    Limit::NumberOfProcesses(8),
                ]))
                .add_config(Config::KeepAlive(vec![
                    AliveCondition::Crashed(true),
                    AliveCondition::OtherJobEnabled({
                        let mut other_jobs = BTreeMap::new();
                        other_jobs.insert(String::from("com.tasker.conflict"), false);
                        other_jobs.insert(String::from("com.tasker.depended"), true);
                        other_jobs
                    }),
                    AliveCondition::SuccessfulExit(false),
                ]))
                .add_config(Config::StartCalendarInterval(vec![
                    CalendarInterval::Hour(9),
                    CalendarInterval::Minute(15),
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
                + "      - NumberOfFiles: 10000\n"
                + "      - NumberOfProcesses: 8\n"
                + "  - KeepAlive:\n"
                + "      - Crashed: true\n"
                + "      - OtherJobEnabled:\n"
                + "          com.tasker.conflict: false\n"
                + "          com.tasker.depended: true\n"
                + "      - SuccessfulExit: false\n"
                + "  - StartCalendarInterval:\n"
                + "      - Hour: 9\n"
                + "      - Minute: 15\n"
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
                + "  - Hour: 9\n"
                + "  - Minute: 15";

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in\n"
                + "  - StartCalendarInterval:\n"
                + "      - Hour: 9\n"
                + "      - Minute: 15";

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
                .add_config(Config::KeepAlive(vec![
                    AliveCondition::Crashed(true),
                    AliveCondition::SuccessfulExit(false),
                ]))
                .add_config(Config::StartCalendarInterval(vec![
                    CalendarInterval::Hour(9),
                    CalendarInterval::Minute(15),
                ]))
                .remove_config("KeepAlive");

            let expected_deserialized = String::new()
                + "---\n"
                + "Label: com.tasker.test_task\n"
                + "Program: /bin/python\n"
                + "Configuration:\n"
                + "  - StandardOutPath: standard_in\n"
                + "  - StartCalendarInterval:\n"
                + "      - Hour: 9\n"
                + "      - Minute: 15";

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
            let mut filtered = raw_plist.split('\n')
                .filter(|&line| !line.starts_with("\t\t<dict>")
                    && !line.starts_with("\t\t</dict>")
                    && !line.starts_with("\t<key>Configuration</key>")
                    && !line.starts_with("\t<array>")
                    && !line.starts_with("\t</array>")
                    && !line.starts_with("\t\t\t\t<dict>")
                    && !line.starts_with("\t\t\t\t</dict>"))
                .map(|line| {line.replace("</array>", "</dict>")})
                .map(|line| {line.replace("<array>", "<dict>")})
                .map(|line| {line.replace("\t\t\t\t\t\t", "\t\t\t\t\t\t\t\t")})
                .map(|line| {line.replace("\t\t\t", "\t")})
                .map(|line| {line.replace("\t\t\t", "\t\t")})
                .collect::<Vec<String>>();
            let mut last_line: Option<&mut String> = None;
            let mut check_wrong_dict_flag: bool = false;
            let mut is_in_wrong_block: bool = false;
            for l in &mut filtered {
                if is_in_wrong_block {
                    if l.starts_with("\t</dict>") {
                        *l = l.replace("\t</dict>", "\t</array>");
                        is_in_wrong_block = false;
                    }
                }
                if check_wrong_dict_flag {
                    if l.starts_with("\t\t<string>") {
                        is_in_wrong_block = true;
                        let get_last_line = last_line.unwrap();
                        *get_last_line = get_last_line.replace("\t<dict>", "\t<array>");

                        last_line = None;
                    }
                    check_wrong_dict_flag = false;
                    continue;
                }
                if l.starts_with("\t<dict>") {
                    check_wrong_dict_flag = true;
                    last_line = Some(l);
                }
            }
            filtered.join("\n")
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
                    + "      - Crashed: true\n"
                    + "      - OtherJobEnabled:\n"
                    + "          com.tasker.conflict: false\n"
                    + "          com.tasker.depended: true\n"
                    + "      - SuccessfulExit: false\n"
                    + "  - StartCalendarInterval:\n"
                    + "      - Hour: 9\n"
                    + "      - Minute: 15\n"
                    + "  - ProgramArguments:\n"
                    + "      - test_script.py\n"
                    + "      - \"--token=12345678\"\n"
                    + "  - EnvironmentVariables:\n"
                    + "      TOKEN: \"12345678\"";

                let expected_plist = String::new()
                    + "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                    + "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"
                    + "<plist version=\"1.0\">\n"
                    + "<dict>\n"
                    + "	<key>Label</key>\n"
                    + "	<string>com.tasker.test_task</string>\n"
                    + "	<key>Program</key>\n"
                    + "	<string>/bin/python</string>\n"
                    + "	<key>StandardOutPath</key>\n"
                    + "	<string>standard_in</string>\n"
                    + "	<key>KeepAlive</key>\n"
                    + "	<dict>\n"
                    + "		<key>Crashed</key>\n"
                    + "		<true />\n"
                    + "		<key>OtherJobEnabled</key>\n"
                    + "		<dict>\n"
                    + "			<key>com.tasker.conflict</key>\n"
                    + "			<false />\n"
                    + "			<key>com.tasker.depended</key>\n"
                    + "			<true />\n"
                    + "		</dict>\n"
                    + "		<key>SuccessfulExit</key>\n"
                    + "		<false />\n"
                    + "	</dict>\n"
                    + "	<key>StartCalendarInterval</key>\n"
                    + "	<dict>\n"
                    + "		<key>Hour</key>\n"
                    + "		<integer>9</integer>\n"
                    + "		<key>Minute</key>\n"
                    + "		<integer>15</integer>\n"
                    + "	</dict>\n"
                    + "	<key>ProgramArguments</key>\n"
                    + "	<array>\n"
                    + "		<string>test_script.py</string>\n"
                    + "		<string>--token=12345678</string>\n"
                    + "	</array>\n"
                    + "	<key>EnvironmentVariables</key>\n"
                    + "	<dict>\n"
                    + "		<key>TOKEN</key>\n"
                    + "		<string>12345678</string>\n"
                    + "	</dict>\n"
                    + "</dict>\n"
                    + "</plist>";

                let config = config::Configuration::from_yaml(&yaml_config).unwrap();

                let plist = get_plist_from_conf(&config);

                assert_eq!(plist, expected_plist);

            }
        }
    }

}
