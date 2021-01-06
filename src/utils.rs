use crate::error::Error;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::macos::fs::MetadataExt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use users::{Group, User};
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

///
/// chown function for path
///
fn chown_by_name(
    path: &Path,
    username: &Option<String>,
    group_name: &Option<String>,
) -> Result<(), Error> {
    let (uid, gid) = get_user_group_pair_id(path, username, group_name)?;
    if let Ok(path) = CString::new(path.as_os_str().as_bytes()) {
        if unsafe { libc::chown(path.as_ptr(), uid, gid) } == 0 {
            return Ok(());
        }
    }
    Err(Error::FailedToChown(format!(
        "failed to change owner ship of `{}`",
        path.to_str().unwrap_or("unknown path")
    )))
}

///
/// recursively change ownership of all included file of a directory
///
pub fn chown_by_name_recursive(
    path: &Path,
    username: &Option<String>,
    group_name: &Option<String>,
) -> Result<(), Error> {
    chown_by_name(path, username, group_name)?;
    if path.is_dir() {
        let mut stack = Vec::new();
        stack.push(PathBuf::from(&path));

        while let Some(working_path) = stack.pop() {
            if let Ok(dir) = std::fs::read_dir(&working_path) {
                for entry in dir {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            stack.push(PathBuf::from(&path));
                        }
                        chown_by_name(&path, username, group_name)?;
                    } else {
                        return Err(Error::FailedToChown(format!(
                            "failed to chown entry: {}",
                            working_path.to_str().unwrap_or("unknown path")
                        )));
                    }
                }
            } else {
                return Err(Error::FailedToChown(format!(
                    "{}",
                    working_path.to_str().unwrap_or("unknown path")
                )));
            }
        }
    }
    Ok(())
}

///
/// Convert `(user name, group name)` to `(user id, group id)` pair,
/// and find primary group if only user is supplied.
/// It return the original uid of the file for uid if only group is supplied.
///
fn get_user_group_pair_id(
    path: &Path,
    username: &Option<String>,
    group_name: &Option<String>,
) -> Result<(u32, u32), Error> {
    if let Ok(meta) = std::fs::metadata(&path) {
        let user: Option<User> = match username {
            None => None,
            Some(name) => match users::get_user_by_name(name) {
                None => None,
                Some(u) => Some(u),
            },
        };

        let group: Option<Group> = match group_name {
            None => None,
            Some(name) => match users::get_group_by_name(name) {
                None => None,
                Some(g) => Some(g),
            },
        };

        match user {
            None => {
                if let Some(g) = group {
                    Ok((meta.st_uid(), g.gid()))
                } else {
                    Ok((meta.st_uid(), meta.st_gid()))
                }
            }
            Some(u) => match group {
                None => Ok((u.uid(), u.primary_group_id())),
                Some(g) => Ok((u.uid(), g.gid())),
            },
        }
    } else {
        Err(Error::PathDoesNotExist(format!(
            "path does not exist in chown"
        )))
    }
}

#[cfg(test)]
mod test_utils_mod {

    use super::*;

    fn create_dir_and_file() -> Result<(), Error> {
        create_dir_check("test")?;
        create_dir_check("test/test_inner_0")?;
        create_dir_check("test/test_inner_1")?;

        let mut dir_vec = Vec::new();
        for d in Path::new("test").read_dir().unwrap() {
            let d = d.unwrap();
            dir_vec.push(d);
        }

        Ok(())
    }

    ///
    /// this test only pass with root user
    ///
    #[test]
    fn chmod_test() -> Result<(), Error> {
        create_dir_and_file()?;
        std::fs::File::create("test/test_inner_1/test.txt").unwrap();
        chown_by_name_recursive(
            Path::new("test"),
            &Some("Congyu WANG".to_string()),
            &None,
        )?;
        // chown_by_name_recursive(
        //     Path::new("/Users/congyuwang/Desktop/tasker_root/tasks/com.tasker.tasks.wubai"),
        //         &Some("Congyu WANG".to_string()),
        //     &None,
        // )?;
        let uid = users::get_user_by_name("Congyu WANG").unwrap().uid();
        let gid = users::get_group_by_name("staff").unwrap().gid();
        assert_eq!(Path::new("test").metadata().unwrap().st_uid(), uid);
        assert_eq!(Path::new("test").metadata().unwrap().st_gid(), gid);
        assert_eq!(Path::new("test/test_inner_1/test.txt").metadata().unwrap().st_uid(), uid);
        assert_eq!(Path::new("test/test_inner_1/test.txt").metadata().unwrap().st_gid(), gid);
        assert_eq!(Path::new("test/test_inner_1").metadata().unwrap().st_uid(), uid);
        assert_eq!(Path::new("test/test_inner_1").metadata().unwrap().st_gid(), gid);
        assert_eq!(Path::new("test/test_inner_0").metadata().unwrap().st_uid(), uid);
        assert_eq!(Path::new("test/test_inner_0").metadata().unwrap().st_gid(), gid);
        std::fs::remove_dir_all("test").unwrap();
        Ok(())
    }
}
