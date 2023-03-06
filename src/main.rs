use colored::Colorize;
use miette::{Context, IntoDiagnostic, Result};
use rashen::format::pman::PmanFile;
use std::{fs, io, path::Path};

// TODO(Unavailable): clap, hexdisplay (nom trait).
// FIX(Unavailable): separate `lib` and `main` dependencies.

fn main() -> Result<()> {
    // FIX(Unavailable): depends on cwd.
    // FIX(Unavailable): message error could be improved.
    let bytes = fs::read(".res/packfile.dat").into_diagnostic()?;
    let pman = PmanFile::new(&bytes)
        .wrap_err_with(|| format!("while parsing the {}.", "packfile.dat".on_red()))?;

    let output_dir = Path::new("output");
    // the directory might not exists, so ignore the error.
    _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir).into_diagnostic()?;

    let size = pman.size_upto_file_data();
    pman.into_iter()
        .try_fold(size, |offset, file| {
            let mut path = output_dir.join(format!("{:08X}", offset));

            if let Some(zlib) = file.to_zlib() {
                path.set_extension("zlib");
                fs::write(path, zlib)?;
            } else {
                path.set_extension("dat");
                fs::write(path, file.bytes())?;
            }

            Ok::<_, io::Error>(offset + file.bytes().len())
        })
        .into_diagnostic()?;

    Ok(())
}
