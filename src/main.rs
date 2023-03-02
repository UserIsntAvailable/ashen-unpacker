use rashen::format::pman::PmanFile;
use std::{
    fs::{self, read},
    io,
    path::Path,
};

fn main() -> eyre::Result<()> {
    // FIX(Unavailable): depends on cwd.
    let bytes = read(".res/packfile.dat")?;
    // FIX(Unavailable): When an error occurs, the user gets a paywall of bits, which is not that
    // useful of a error message.
    let pman = PmanFile::new(bytes)?;

    let output_dir = Path::new("output");
    // the directory might not exists, so ignore the error.
    _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir)?;

    let size = pman.size_upto_file_data();
    pman.into_iter().try_fold(size, |offset, file| {
        let mut path = output_dir.join(format!("{:08X}", offset));

        if let Some(zlib) = file.to_zlib() {
            path.set_extension("zlib");
            fs::write(path, zlib)?;
        } else {
            path.set_extension("dat");
            fs::write(path, file.bytes())?;
        }

        Ok::<_, io::Error>(offset + file.bytes().len())
    })?;

    Ok(())
}
