use rashen::format::pman::PmanFile;
use std::{
    fs::{self, read},
    path::Path,
};

fn main() -> eyre::Result<()> {
    let bytes = read(".res/packfile.dat")?;
    let pman = PmanFile::new(bytes)?;

    let output_dir = Path::new("output");
    // the directory might not exists, so ignore the error.
    _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir)?;

    pman.files()
        .iter()
        .try_fold(pman.files_start_offset(), |offset, file| {
            let mut path = output_dir.join(format!("{:08X}", offset));

            if let Some(zlib) = file.to_zlib() {
                path.set_extension("zlib");
                fs::write(path, zlib)?;
            } else {
                path.set_extension("dat");
                fs::write(path, file.bytes())?;
            }

            Ok::<_, std::io::Error>(offset + file.bytes().len())
        })?;

    Ok(())
}
