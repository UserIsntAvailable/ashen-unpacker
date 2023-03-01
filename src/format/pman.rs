use flate2::read::ZlibDecoder;
use nom::{
    bytes::complete::{tag, take},
    multi::count,
    number::complete::le_u32,
    IResult,
};
use std::io::Read;
use std::mem::size_of;

type Result<'a, T> = IResult<&'a [u8], T>;

#[allow(dead_code)]
enum FileType {
    Unknown,
    /// Information about the current map being played, could be from main
    /// campaign or multiplayer.
    Level,
    /// COLL
    Collision,
    /// TWPT
    Waypoint,
    /// The color of the textures.
    Color,
    /// utf-16 text
    Language,
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
            // FIX: Check if enough bytes were read.
            // FIX: Error handling
            decoder.read_to_end(&mut zlib).unwrap();

            zlib
        })
    }
}

const HEADER_SEP: [u8; 0x10] = [0; 0x10];

fn parse_header(input: &[u8]) -> Result<(String, u32)> {
    let (input, _) = tag("PMAN")(input)?;
    let (input, file_count) = le_u32(input)?;
    let (input, copyright) = take(0x28u8)(input)?;
    let (input, _) = tag(HEADER_SEP)(input)?;

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

// TODO: There is probably a better way of doing this.
//
// The offset value of a entry is not always prev_offset + prev_size. Idk why that could be the
// case, but oh well it is what it is.
fn parse_files<'a>(
    input: &'a [u8],
    entry_table: &'a [u8],
    file_count: u32,
) -> Result<'a, Vec<PmanFileData>> {
    let file_count = file_count as usize;
    let offset = u32::from_le_bytes(entry_table[4..8].try_into().unwrap());

    let (_, files) = count(
        |entry_table| {
            let (entry_table, u32) = count(le_u32, 4)(entry_table)?;
            let offset = u32[1] as usize - offset as usize;
            let data = &input[offset..offset + u32[2] as usize];

            Ok((
                entry_table,
                PmanFileData {
                    // TODO: Automatic zlib decompression.
                    bytes: data.to_vec(),
                },
            ))
        },
        file_count,
    )(entry_table)?;

    Ok((&input[input.len() - 4..], files))
}

pub struct PmanFile {
    copyright: String,
    files: Vec<PmanFileData>,
}

// TODO: impl IntoIterator
// TODO: impl Index
impl PmanFile {
    pub fn new(bytes: Vec<u8>) -> eyre::Result<PmanFile> {
        // needed to infer the err case of `?`.
        fn _new(bytes: &[u8]) -> Result<PmanFile> {
            let (input, (copyright, file_count)) = parse_header(bytes)?;
            let (input, entry_table) = parse_entry_table(input, file_count)?;
            let (input, files) = parse_files(input, entry_table, file_count)?;
            let (input, _) = tag("Fmom")(input)?;

            Ok((input, PmanFile { copyright, files }))
        }

        // FIX: When an error occurs, the user gets a paywall of bits, which is not that useful as a
        // error message.
        Ok(_new(&bytes).map_err(|err| err.map_input(<[u8]>::to_vec))?.1)
    }

    pub fn files_start_offset(&self) -> usize {
        let header = 4 + size_of::<u32>() + self.copyright.len() + HEADER_SEP.len();
        let entry_table = get_entry_table_size(self.files.len() as u32);

        header + entry_table
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
    fn file_test() -> eyre::Result<()> {
        _ = PmanFile::new(INPUT.to_vec())?;

        Ok(())
    }
}
