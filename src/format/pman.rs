use flate2::read::ZlibDecoder;
use nom::{
    bytes::complete::{tag, take},
    multi::fill,
    number::complete::le_u32,
    IResult,
};
use std::{
    io::{self, Read, Write},
    mem::size_of,
    ops::Index,
};

type Result<'a, T> = IResult<&'a [u8], T>;

#[allow(dead_code)]
enum PmanFileType {
    Unknown,
    /// Information about the current map being played.
    Level,
    /// COLL
    Collision,
    /// TWPT
    Waypoint,
    /// The color palette used for textures.
    Palette,
    /// Mainly for different language text. UTF-16
    Text,
}

pub struct PmanFileData {
    bytes: Vec<u8>,
    // r#type: FileType TODO: Data detection.
}

impl PmanFileData {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }

    pub fn to_zlib(&self) -> Option<Vec<u8>> {
        (self.bytes[..2] == [b'Z', b'L']).then(|| {
            let size = u32::from_le_bytes([self.bytes[2], self.bytes[3], self.bytes[4], 0]);
            let mut decoder = ZlibDecoder::new(&self.bytes[5..]);
            let mut zlib = Vec::<u8>::with_capacity(size as usize);
            // FIX: Error handling:
            // - Check if enough bytes were read.
            // - Remove unwrap.
            decoder.read_to_end(&mut zlib).unwrap();

            zlib
        })
    }
}

const HEADER_MAGIC_STRING: [u8; 4] = [b'P', b'M', b'A', b'N'];
const HEADER_SEPARATOR: [u8; 0x10] = [0; 0x10];

fn parse_header(input: &[u8]) -> Result<(String, u32)> {
    let (input, _) = tag(HEADER_MAGIC_STRING)(input)?;
    let (input, file_count) = le_u32(input)?;
    let (input, copyright) = take(0x28u8)(input)?;
    let (input, _) = tag(HEADER_SEPARATOR)(input)?;

    Ok((
        input,
        (String::from_utf8_lossy(copyright).into(), file_count),
    ))
}

fn get_entry_table_size(file_count: u32) -> usize {
    file_count as usize * size_of::<u32>() * 4
}

fn parse_entry_table(input: &[u8], file_count: u32) -> Result<&[u8]> {
    take(get_entry_table_size(file_count))(input)
}

fn parse_files<'a>(
    input: &'a [u8],
    entry_table: &'a [u8],
    file_count: u32,
) -> Result<'a, Vec<PmanFileData>> {
    let file_count = file_count as usize;

    fn parse_entry(entry_table: &[u8]) -> Result<(usize, usize)> {
        let mut fields = [0; 4];
        let (entry_table, ()) = fill(le_u32, &mut fields)(entry_table)?;
        // FIX: assert that fields[0] and fields[3] should be 0.

        Ok((entry_table, (fields[1] as usize, fields[2] as usize)))
    }

    let (_, (file_offset, _)) = parse_entry(entry_table)?;
    let files = Vec::with_capacity(file_count);

    let (input, _, _, files) = (0..file_count).try_fold(
        (input, entry_table, (file_offset, 0), files),
        |(input, entry_table, (prev_offset, prev_size), mut files), _| {
            let (entry_table, entry) = parse_entry(entry_table)?;
            let (input, _) = take(entry.0 - (prev_offset + prev_size))(input)?;
            let (input, data) = take(entry.1)(input)?;

            files.push(PmanFileData {
                bytes: data.to_vec(),
            });

            Ok((input, entry_table, entry, files))
        },
    )?;

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
            let (input, (copyright, file_count)) = parse_header(bytes)?;
            let (input, entry_table) = parse_entry_table(input, file_count)?;
            let (input, files) = parse_files(input, entry_table, file_count)?;
            // FIX: assert input is empty.

            Ok((input, PmanFile { copyright, files }))
        }

        // FIX: When an error occurs, the user gets a paywall of bits, which is not that useful as a
        // error message.
        Ok(_new(&bytes).map_err(|err| err.map_input(<[u8]>::to_vec))?.1)
    }

    pub fn copyright(&self) -> &str {
        &self.copyright
    }

    pub fn files(&self) -> &[PmanFileData] {
        &self.files
    }

    pub fn copyright_mut(&mut self) -> &mut String {
        &mut self.copyright
    }

    pub fn files_mut(&mut self) -> &mut Vec<PmanFileData> {
        &mut self.files
    }

    pub fn size_upto_file_data(&self) -> usize {
        let header = 4 + size_of::<u32>() + self.copyright.len() + HEADER_SEPARATOR.len();
        let entry_table = get_entry_table_size(self.files.len() as u32);

        header + entry_table
    }

    /// Turns this `PmanFile` back to its bytes representation.
    pub fn into_bytes(self) -> io::Result<Vec<u8>> {
        let files_size = self.files.iter().map(|f| f.bytes().len()).sum::<usize>();
        let size = self.size_upto_file_data();

        // FIX: Could potentially fail if size + files_size >= isize::MAX;
        let mut buf = Vec::with_capacity(size + files_size);

        buf.write_all(&HEADER_MAGIC_STRING)?;
        buf.write_all(&(self.files.len() as u32).to_le_bytes())?;
        buf.write_all(&self.copyright.as_bytes())?;
        buf.write_all(&HEADER_SEPARATOR)?;

        let zero_bytes = 0u32.to_le_bytes();
        // FIX: as u32 is not safe if size is bigger that u32::MAX.
        // Realistically speaking that is unlikely, but I should be more explicit with what an
        // invalid file should look like.
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
    fn header_test() -> eyre::Result<()> {
        let (_, (copyright, file_count)) = parse_header(INPUT)?;

        assert_eq!(copyright, "Copyright (c) 2004 Torus Games Pty. Ltd.");
        assert_eq!(file_count, FILE_COUNT);

        Ok(())
    }

    #[test]
    fn entry_table_test() -> eyre::Result<()> {
        let (_, entry_table) = parse_entry_table(&INPUT[ENTRY_TABLE_START..], FILE_COUNT)?;
        let first_entry: [u32; 4] = {
            let slice = &entry_table[..size_of::<u32>() * 4];

            bytemuck::cast::<[u8; 16], _>(slice.try_into().unwrap())
        };

        assert_eq!(first_entry, [0, 0xA20, 0x6500, 0]);

        Ok(())
    }

    #[test]
    fn files_test() -> eyre::Result<()> {
        let first_file = &INPUT[0xA20..];
        let entry_table = &INPUT[ENTRY_TABLE_START..];

        let (_, files) = parse_files(first_file, entry_table, FILE_COUNT)?;
        let file = files[77].to_zlib().expect("zlib file data.");

        assert_eq!(file[..4], [b'C', b'O', b'L', b'L']);

        Ok(())
    }

    #[test]
    fn pman_new_test() -> eyre::Result<()> {
        _ = PmanFile::new(INPUT.to_vec())?;

        Ok(())
    }

    #[test]
    fn into_bytes_test() -> eyre::Result<()> {
        let pman = PmanFile::new(INPUT.to_vec())?;
        let bytes = &pman.into_bytes()?;

        assert_eq!(bytes.len() + 170, INPUT.len());

        Ok(())
    }
}
