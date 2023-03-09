pub mod pman;

pub(super) type Result<'a, T> = nom::IResult<&'a [u8], T>;

/// Reads an `u32` and verifies if it is zero.
pub(super) fn u32_zero(input: &[u8]) -> Result<u32> {
    use nom::{combinator::verify, number::complete::le_u32};

    verify(le_u32, |x| *x == 0)(input)
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileEntry {
    pub offset: usize,
    pub size: usize,
}

impl FileEntry {
    pub(super) fn new(offset: u32, size: u32) -> Self {
        Self {
            offset: offset as usize,
            size: size as usize,
        }
    }

    pub(super) fn with_size(mut self, size: u32) -> Self {
        self.size = size as usize;

        self
    }

    pub(super) fn from_bytes(input: &[u8]) -> Result<Self> {
        #[rustfmt::skip]
        use nom::{
            multi::fill,
            number::complete::le_u32,
        };

        let mut fields = [0; 2];
        // let (input, _) = terminated(fill(le_u32, &mut fields), u32_zero)(input)?;
        let (input, _) = fill(le_u32, &mut fields)(input)?;

        Ok((input, FileEntry::new(fields[0], fields[1])))
    }
}
