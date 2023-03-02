pub mod pman;

pub(crate) type Result<'a, T> = nom::IResult<&'a [u8], T>;
