pub mod pman;

use crate::error::NomResult;
use nom::{multi::fill, number::complete::le_u32};

#[derive(Debug, Default, Copy, Clone)]
pub(super) struct Entry {
    pub(super) offset: u32,
    pub(super) size: u32,
}

impl Entry {
    pub(super) fn new(offset: u32, size: u32) -> Self {
        Self { offset, size }
    }

    pub(super) fn from_bytes(input: &[u8]) -> NomResult<Self> {
        let mut fields = [0; 3];
        let (input, ()) = fill(le_u32, &mut fields)(input)?;
        // FIX(Unavailable): assert that fields[2] should be 0.

        Ok((
            input,
            Entry {
                offset: fields[0],
                size: fields[1],
            },
        ))
    }
}
