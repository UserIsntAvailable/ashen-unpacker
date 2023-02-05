use crate::format::PmanFile;
use format::BinaryChunk;
use std::{fs, path::Path};

mod format;

fn main() {
    let buffer = include_bytes!("../packfile.dat");

    match PmanFile::new_read(buffer, &mut 0) {
        Ok(file) => {
            let path_dir = Path::new("output");
            let _ = fs::remove_dir_all(path_dir);
            fs::create_dir_all(path_dir).expect("Cannot create an output directory");
            for (declaration, file) in file.file_declarations.iter().zip(file.files) {
                let path = path_dir.join(format!("{:X}.dat", declaration.offset));
                fs::write(&path, &file.data).expect("Could not write a data file");
                if file.is_zlib() {
                    let path = path_dir.join(format!("{:X}.zlib", declaration.offset));
                    let data = file.zlib_data().expect("Invalid ZLIB archive");
                    fs::write(&path, &data).expect("Could not write a data file");
                }
            }
        }
        Err(e) => println!("{:?}", e),
    }
}
