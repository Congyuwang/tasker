/// config module provides general configuration for tasks
pub mod config {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// Config:
    /// This enum is used to be directly mapped to launchd.plist XML.
    /// It is not meant to be directly used by library user.<br>
    /// <p>
    /// Notes:
    /// The Program key must be an absolute path.
    /// </p>
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct Configuration {
        label: String,
        program: String,
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

        fn add_config(mut self, config: Config) -> Configuration {
            self.configuration.push(config);
            self
        }
    }

    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    enum Config {
        ProgramArguments(Option<Vec<String>>),
        EnvironmentVariables(Option<HashMap<String, String>>),
        KeepAlive(Option<Vec<AliveCondition>>),
        RunAtLoad(Option<bool>),
        WorkingDirectory(Option<String>),
        ExitTimeOut(Option<i32>),
        StartInterval(Option<i32>),
        StartCalendarInterval(Option<Vec<CalendarInterval>>),
        StandardInPath(Option<String>),
        StandardOutPath(Option<String>),
        StandardErrorPath(Option<String>),
        SoftResourceLimit(Option<Vec<Limit>>),
        HardResourceLimits(Option<Vec<Limit>>),
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
    /// <li>Crashed (boolean):<br>
    /// If true, the the job will be restarted as long as it exited due to a signal which is typically
    /// associated with a crash (SIGILL, SIGSEGV, etc.). If false, the job will be restarted in the
    /// inverse condition.</li>
    ///
    /// </ul>
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    enum AliveCondition {
        Always,
        SuccessfulExit(bool),
        OtherJobEnabled(HashMap<String, bool>),
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
    enum CalendarInterval {
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
    enum Limit {
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
                .add_config(Config::StandardOutPath(Some(
                    "standard_in".parse().unwrap(),
                )))
                .add_config(Config::HardResourceLimits(Some(vec![
                    Limit::NumberOfFiles(10000),
                    Limit::NumberOfProcesses(8),
                ])))
                .add_config(Config::KeepAlive(Some(vec![
                    AliveCondition::Crashed(true),
                    AliveCondition::SuccessfulExit(false),
                ])))
                .add_config(Config::StartCalendarInterval(Some(vec![
                    CalendarInterval::Hour(9),
                    CalendarInterval::Minute(15),
                ])));

            let expected_deserialized = String::new()
                + "---\n"
                + "label: com.tasker.test_task\n"
                + "program: /bin/python\n"
                + "configuration:\n"
                + "  - StandardOutPath: standard_in\n"
                + "  - HardResourceLimits:\n"
                + "      - NumberOfFiles: 10000\n"
                + "      - NumberOfProcesses: 8\n"
                + "  - KeepAlive:\n"
                + "      - Crashed: true\n"
                + "      - SuccessfulExit: false\n"
                + "  - StartCalendarInterval:\n"
                + "      - Hour: 9\n"
                + "      - Minute: 15";

            assert_eq!(
                expected_deserialized,
                serde_yaml::to_string(&test_config).unwrap()
            );

            assert_eq!(
                test_config,
                serde_yaml::from_str::<Configuration>(&expected_deserialized[..]).unwrap()
            );
        }
    }
}

pub mod launchd {}
