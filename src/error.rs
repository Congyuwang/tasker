use std::{io, string};

#[derive(Debug)]
pub enum Error {
    YamlError(String),
    ConfigRangeError(String),
    ConfigPathError(String),
    ConfigLabelError(String),
    ConfigProgramError(String),
}
