use crate::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip;

pub fn create_dir_check<P: AsRef<Path>>(dest: P) -> Result<(), Error> {
    if std::fs::metadata(&dest).is_err() {
        return match std::fs::create_dir_all(&dest) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ErrorCreatingFolder(format!(
                "Folder cannot be created"
            ))),
        };
    }
    Ok(())
}

pub fn create_file_check<P: AsRef<Path>>(dest: P) -> Result<(), Error> {
    if std::fs::metadata(&dest).is_err() {
        return match std::fs::File::create(&dest) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ErrorCreatingFolder(format!(
                "File cannot be created"
            ))),
        };
    }
    Ok(())
}

pub fn delete_file_check<P: AsRef<Path>>(dest: P) -> Result<(), Error> {
    return if !std::fs::metadata(&dest).is_err() {
        match std::fs::remove_file(&dest) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::NoFileToDelete(String::from(
                "no file found to delete",
            ))),
        }
    } else {
        Err(Error::NoFileToDelete(String::from(
            "no file found to delete",
        )))
    };
}

pub fn execute_command(command: &mut Command) -> Result<String, Error> {
    let output = command.output();
    let output = match output {
        Ok(o) => o,
        Err(_) => {
            return Err(Error::CommandExecutionError("unknown error".to_string()));
        }
    };
    if !output.status.success() {
        return Err(Error::CommandExecutionError(format!(
            "failed to execute command: {}",
            std::str::from_utf8(&output.stderr).unwrap()
        )));
    };
    if let Ok(output) = std::str::from_utf8(&output.stdout) {
        Ok(output.to_string())
    } else {
        Err(Error::CommandExecutionError(
            "non-utf8 output not supported".to_string(),
        ))
    }
}

pub fn decompress(zip_path: &Path, out_dir: &Path) -> Result<(), Error> {
    if let Ok(zip_file) = File::open(zip_path) {
        if let Ok(mut zip) = zip::ZipArchive::new(zip_file) {
            match create_dir_check(&out_dir) {
                Ok(_) => {
                    for i in 0..zip.len() {
                        if let Ok(mut f) = zip.by_index(i) {
                            if f.name().starts_with("__MACOSX") {
                                continue;
                            }
                            let new_path = out_dir.join(f.name());
                            if f.is_dir() {
                                match create_dir_check(&new_path) {
                                    Ok(_) => {}
                                    Err(_) => {
                                        return Err(Error::DecompressionError(
                                            "decompression failure".parse().unwrap(),
                                        ))
                                    }
                                };
                            } else if f.is_file() {
                                if let Ok(mut outfile) = std::fs::File::create(&new_path) {
                                    std::io::copy(&mut f, &mut outfile).unwrap();
                                } else {
                                    return Err(Error::DecompressionError(
                                        "decompression failure".parse().unwrap(),
                                    ));
                                }
                            }
                        } else {
                            return Err(Error::DecompressionError(
                                "decompression failure".parse().unwrap(),
                            ));
                        }
                    }
                }
                Err(_) => {
                    return Err(Error::DecompressionError(
                        "failed to create decompression folder".parse().unwrap(),
                    ))
                }
            };
        } else {
            return Err(Error::DecompressionError(
                "failed to decompress zip archive".parse().unwrap(),
            ));
        }
    } else {
        return Err(Error::DecompressionError(
            "failed to open zip file".parse().unwrap(),
        ));
    };
    Ok(())
}

pub fn read_utf8_file(file: &Path) -> std::io::Result<String> {
    let mut file = File::open(file)?;
    let mut utf8_string = String::new();
    file.read_to_string(&mut utf8_string)?;
    Ok(utf8_string)
}

///
/// this function moves files in a folder recursively using rename method.
///
pub fn move_by_rename(from: &Path, to: &Path) -> Result<(), Error> {
    match move_by_rename_inner(from, to) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::RenameError(format!(
            "error moving from {} to {}: {}",
            from.display(),
            to.display(),
            e.to_string()
        ))),
    }
}

fn create_dir_io_error(dir: &Path) -> Result<(), std::io::Error> {
    match create_dir_check(&dir) {
        Ok(_) => Ok(()),
        Err(_) => return Err(std::io::Error::from(std::io::ErrorKind::Other)),
    }
}

fn move_by_rename_inner(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    create_dir_io_error(&to)?;
    let from = std::path::Path::new(from);
    let to = std::path::Path::new(to);
    let mut stack = Vec::new();
    stack.push(PathBuf::from(&from));

    let output_root = PathBuf::from(&to);
    let input_root = PathBuf::from(&from).components().count();

    while let Some(working_path) = stack.pop() {
        // relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        create_dir_io_error(&to)?;

        for entry in std::fs::read_dir(working_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        let dest_path = dest.join(filename);
                        std::fs::rename(&path, &dest_path)?;
                    }
                    None => {}
                }
            }
        }
    }

    std::fs::remove_dir_all(from).unwrap();

    Ok(())
}
