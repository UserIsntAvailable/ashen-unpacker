use super::{FileEntry, Result};
use flate2::read::ZlibDecoder;
use nom::{
    bytes::complete::{tag, take},
    character::complete::char,
    combinator::eof,
    multi::{count, separated_list1},
    number::complete::le_u32,
    sequence::{preceded, terminated, Tuple},
};
use std::{
    io::{self, Read, Write},
    mem::size_of,
    ops::Index,
};

// TODO(Unavailable): Rename to symbols (functions/variables) from debug build.

#[allow(dead_code)]
enum PmanFileType {
    Unknown,
    Entity,
    Skybox,
    /// Information about the current map being played.
    Level,
    /// COLL
    Collision,
    /// TWPT
    Waypoint,
    /// The color palette used for textures.
    Palette,
    /// Mainly for language text banks. UTF-16
    Text,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PmanFileData {
    bytes: Vec<u8>,
    // r#type: PmanFileType TODO(Unavailable): Data detection.
}

impl PmanFileData {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }

    pub fn to_zlib(&self) -> Option<Vec<u8>> {
        // TODO(Unavailable): use nom.

        (&self.bytes[..2] == b"ZL").then(|| {
            let size = u32::from_le_bytes([self.bytes[2], self.bytes[3], self.bytes[4], 0]);
            let mut decoder = ZlibDecoder::new(&self.bytes[5..]);
            let mut zlib = Vec::<u8>::with_capacity(size as usize);
            // FIX(Unavailable): Error handling:
            // - Check if enough bytes were read.
            // - Remove unwrap.
            decoder.read_to_end(&mut zlib).unwrap();

            zlib
        })
    }
}

const HEADER_SIZE: usize = 64;
const HEADER_MAGIC_STRING: &[u8; 4] = b"PMAN";
const COPYRIGHT_MAX_SIZE: usize = HEADER_SIZE - 9;

fn read_header(input: &[u8]) -> Result<(String, u32)> {
    const NULL: char = '\0';

    let (input, header) = take(HEADER_SIZE)(input)?;
    let (header, _) = tag(HEADER_MAGIC_STRING)(header)?;
    let (header, file_entry_count) = le_u32(header)?;
    let (header, copyright) = terminated(take(COPYRIGHT_MAX_SIZE), char(NULL))(header)?;
    _ = eof(header)?; // Not really needed, but having a guard doesn't hurt.

    let copyright = String::from_utf8_lossy(copyright);
    let copyright = copyright.trim_end_matches(NULL);

    Ok((input, (copyright.into(), file_entry_count)))
}

fn entry_table_size(file_entry_count: u32) -> usize {
    file_entry_count as usize * size_of::<u32>() * 4
}

fn read_file_entries(input: &[u8], file_entry_count: u32) -> Result<Vec<FileEntry>> {
    let (input, table) = take(entry_table_size(file_entry_count))(input)?;
    let (table, entries) = separated_list1(super::u32_zero, |table| {
        // TODO(Unavailable): On the original source code this u32 should be the file type, but for
        // some reason on the version `1.0.6` this value is always `0`.
        //
        // I could rewrite the `packfile.dat` to add this values by default.
        let (table, _type) = super::u32_zero(table)?;

        FileEntry::from_bytes(table)
    })(table)?;
    _ = ((super::u32_zero, eof)).parse(table)?;

    Ok((input, entries))
}

fn read_files<'a>(input: &'a [u8], file_entries: Vec<FileEntry>) -> Result<'a, Vec<PmanFileData>> {
    let len = file_entries.len();

    // TODO(Unavailable): Can this be improved further?
    let mut prev_entry = file_entries.first().unwrap().with_size(0);
    let mut iter = file_entries.into_iter();

    let (input, files) = count(
        |input: &'a [u8]| {
            let entry = iter.next().unwrap();
            let FileEntry {
                offset: prev_offset,
                size: prev_size,
            } = prev_entry;

            let (input, data) = preceded(
                // reads the extra NULL bytes from previous offset.
                take(entry.offset - (prev_offset + prev_size)),
                take(entry.size),
            )(input)?;

            prev_entry = entry;

            Ok((
                input,
                PmanFileData {
                    bytes: data.to_vec(),
                },
            ))
        },
        len,
    )(input)?;

    Ok((input, files))
}

pub struct PmanFile {
    copyright: String,
    files: Vec<PmanFileData>,
}

impl PmanFile {
    pub fn new(bytes: Vec<u8>) -> eyre::Result<PmanFile> {
        // needed to infer the err case of `?`.
        fn _new(bytes: &[u8]) -> Result<PmanFile> {
            let (input, (copyright, entry_count)) = read_header(bytes)?;
            let (input, entries) = read_file_entries(input, entry_count)?;
            let (input, files) = read_files(input, entries)?;
            // FIX(Unavailable): assert input is empty.

            Ok((input, PmanFile { copyright, files }))
        }

        Ok(_new(&bytes).map_err(|err| err.map_input(<[u8]>::to_vec))?.1)
    }

    pub fn copyright(&self) -> &str {
        &self.copyright
    }

    /// Sets the copyright notice of the `PmanFile`.
    ///
    /// # Panics
    ///
    /// If the new copyright string length is `>` than 55. Do note that `length` != `# of chars`;
    /// you can read [`String::len`] for more information.
    pub fn set_copyright<S>(&mut self, copyright: S)
    where
        S: Into<String>,
    {
        let copyright = copyright.into();

        assert!(
            copyright.len() <= COPYRIGHT_MAX_SIZE,
            "copyright notice should be less than {COPYRIGHT_MAX_SIZE} bytes long."
        );

        self.copyright = copyright;
    }

    pub fn files(&self) -> &[PmanFileData] {
        &self.files
    }

    pub fn files_mut(&mut self) -> &mut Vec<PmanFileData> {
        &mut self.files
    }

    pub fn size_upto_file_data(&self) -> usize {
        HEADER_SIZE + entry_table_size(self.files.len() as u32)
    }

    /// Turns this `PmanFile` back to its bytes representation.
    pub fn into_bytes(self) -> io::Result<Vec<u8>> {
        // TODO(Unavailable): I can probably remove all `?` with unwraps...

        let files_size = self.files.iter().map(|f| f.bytes().len()).sum::<usize>();
        let size = self.size_upto_file_data();

        // FIX(Unavailable): Could potentially fail if size + files_size >= isize::MAX;
        //
        // Could be mitigated by converting `files_mut` into `set_files` and then check invalid
        // state beforehand.
        let mut buf = Vec::with_capacity(size + files_size);

        buf.write_all(HEADER_MAGIC_STRING)?;
        buf.write_all(&(self.files.len() as u32).to_le_bytes())?;
        buf.write_all(&self.copyright.as_bytes())?;

        let zero_bytes = [0; 1];
        // + 1 to include a null character.
        (0..COPYRIGHT_MAX_SIZE - self.copyright.len() + 1)
            .try_for_each(|_| buf.write_all(&zero_bytes))?;

        let zero_bytes = [0; 4];
        // FIX(Unavailable): `as u32` is not safe if size is bigger that u32::MAX.
        //
        // Realistically speaking that is unlikely to happen, but I should be more explicit with
        // what an invalid file should look like.
        self.files.iter().try_fold(size as u32, |offset, file| {
            let size = file.bytes.len() as u32;

            buf.write_all(&zero_bytes)?;
            buf.write_all(&offset.to_le_bytes())?;
            buf.write_all(&size.to_le_bytes())?;
            buf.write_all(&zero_bytes)?;

            Ok::<_, io::Error>(offset + size)
        })?;

        self.into_iter()
            .try_for_each(|file| buf.write_all(file.bytes()))?;

        Ok(buf)
    }
}

impl IntoIterator for PmanFile {
    type Item = PmanFileData;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.files.into_iter()
    }
}

impl Index<usize> for PmanFile {
    type Output = PmanFileData;

    fn index(&self, index: usize) -> &Self::Output {
        &self.files[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &'static [u8] = include_bytes!("../../.res/packfile.dat");
    const FILE_COUNT: u32 = 158;
    const ENTRY_TABLE_START: usize = 0x40;

    #[test]
    fn read_header_test() -> eyre::Result<()> {
        let (_, (copyright, file_count)) = read_header(INPUT)?;

        assert_eq!(copyright, "Copyright (c) 2004 Torus Games Pty. Ltd.");
        assert_eq!(file_count, FILE_COUNT);

        Ok(())
    }

    #[test]
    fn read_entry_table_test() -> eyre::Result<()> {
        let (_, entry_table) = read_file_entries(&INPUT[ENTRY_TABLE_START..], FILE_COUNT)?;

        assert_eq!(*entry_table.first().unwrap(), FileEntry::new(0xA20, 0x6500));

        Ok(())
    }

    #[test]
    fn read_files_test() -> eyre::Result<()> {
        let (input, entry_table) = read_file_entries(&INPUT[ENTRY_TABLE_START..], FILE_COUNT)?;
        let (_, files) = read_files(input, entry_table)?;
        let file = files[77].to_zlib().expect("zlib file data.");

        assert_eq!(&file[..4], b"COLL");

        Ok(())
    }

    #[test]
    fn pman_new_test() -> eyre::Result<()> {
        _ = PmanFile::new(INPUT.to_vec())?;

        Ok(())
    }

    #[test]
    fn pman_into_bytes_test() -> eyre::Result<()> {
        let pman = PmanFile::new(INPUT.to_vec())?;
        let bytes = &pman.into_bytes()?;

        assert_eq!(bytes.len() + 170, INPUT.len());

        Ok(())
    }
}
