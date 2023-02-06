mod format;

use crate::format::PmanFile;
use format::BinaryChunk;
use std::{fs, path::Path};

fn main() -> eyre::Result<()> {
    let file_bytes = include_bytes!("../.res/packfile.dat");
    let mut offset = 0;
    let pman_file = PmanFile::new_read(file_bytes, &mut offset)
        .expect("packfile.dat should be exists and be a valid ashen file.");

    let output_dir = Path::new("output");
    // the directory might not exists, so ignore it.
    _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir)?;

    for (declaration, file) in pman_file.file_declarations.iter().zip(pman_file.files) {
        let mut path = output_dir.join(format!("{:X}", declaration.offset));

        if !file.is_zlib() {
            path.set_extension("dat");
            fs::write(path, file.data)?;
        } else {
            path.set_extension("zlib");
            fs::write(path, file.zlib_data().expect("Invalid ZLIB archive"))?;
        }
    }

    println!("Current file offset {offset:X}");

    Ok(())
}
