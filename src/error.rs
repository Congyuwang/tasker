use std::{io, string};

#[derive(Debug)]
pub enum Error {
    YamlError(serde_yaml::Error),
    ConfigRangeError(String),
    ConfigPathError(String),
    ConfigLabelError(String),
    ConfigProgramError(String),
    LaunchctlListError(String),
    DecompressionError(String),
    RenameError(String),
}
