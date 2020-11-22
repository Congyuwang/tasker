use zip;
use std::fs::File;
use std::path::{PathBuf, Path};
use crate::error::Error;
use zip::ZipArchive;
use zip::result::ZipError;

pub fn decompress(zip_path: &str, out_dir: &str) -> Result<(), Error> {
    match decompress_inner(zip_path, out_dir) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::DecompressionError("decompression failure".parse().unwrap()))
    }
}

fn create_dir_check<P: AsRef<Path>>(dest: P) {
    if std::fs::metadata(&dest).is_err() {
        std::fs::create_dir_all(&dest);
    }
}

fn decompress_inner(zip_path: &str, out_dir: &str) -> Result<(), ZipError> {
    let mut out_dir = String::from(out_dir);
    if !out_dir.ends_with('/') {
        out_dir = out_dir + "/";
    }
    let zip_file = File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(zip_file)?;
    create_dir_check(&out_dir);
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.name().starts_with("__MACOSX") {
            continue;
        }
        let new_path = String::from("") + &out_dir + f.name();
        if f.is_dir() {
            create_dir_check(&new_path);
        } else if f.is_file() {
            let mut outfile = std::fs::File::create(&new_path)?;
            std::io::copy(&mut f, &mut outfile).unwrap();
        }
    }
    Ok(())
}

///
/// this function moves files in a folder recursively using rename method.
///
pub fn move_by_rename(from: &str, to: &str) -> Result<(), Error> {
    match move_by_rename_inner(from, to) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::RenameError(format!("error moving from {} to {}: {}", from , to, e.to_string())))
    }
}


fn move_by_rename_inner(from: &str, to: &str) -> Result<(), std::io::Error> {
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
        create_dir_check(&dest);

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

    std::fs::remove_dir_all(from);

    Ok(())
}


#[cfg(test)]
mod util_test {
    use super::*;

    #[test]
    fn decompress_test() {
        match decompress("/Users/congyuwang/Desktop/test_dir/zip.zip",
                         "/Users/congyuwang/Desktop/test_dir/out") {
            Ok(_) => {}
            Err(m) => {println!("{:?}", m)}
        };
    }

    #[test]
    fn move_test() {
        match move_by_rename("/Users/congyuwang/Desktop/test_dir/out/",
                             "/Users/congyuwang/Desktop/out/") {
            Ok(_) => {}
            Err(m) => {println!("{:?}", m)}
        }
    }
}

