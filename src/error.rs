use miette::Diagnostic;
use thiserror::Error;

// TODO(Unavailable): Should I keep this alias?
//
// There are really few places where this Result type will be used; I could rename `NomResult` to
// `Result`, since it is used all over the place.
pub type Result<T> = std::result::Result<T, Error>;

type Input<'a> = &'a [u8];
pub(crate) type NomError<'a> = nom_supreme::error::ErrorTree<Input<'a>>;
pub(crate) type NomResult<'a, T> = nom::IResult<Input<'a>, T, NomError<'a>>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(code(rashen::io_error))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(rashen::invalid_format))]
    InvalidFormat(#[from] InvalidFormatError),
}

impl From<NomError<'_>> for Error {
    fn from(value: NomError<'_>) -> Self {
        Error::from(InvalidFormatError::from(value))
    }
}

#[derive(Error, Diagnostic, Debug)]
pub struct InvalidFormatError {
    #[source_code]
    input: String,
    #[help]
    help: String,
}

impl From<NomError<'_>> for InvalidFormatError {
    fn from(_value: NomError<'_>) -> Self {
        todo!()
    }
}

impl std::fmt::Display for InvalidFormatError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
