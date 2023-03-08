use miette::{Diagnostic, MietteError, SourceCode, SourceSpan, SpanContents};
use nom_supreme::error::GenericErrorTree;
use std::fmt::Display;
use thiserror::Error;

// TODO(Unavailable): Should I keep this alias?
//
// There are really few places where this Result type will be used; I could rename `NomResult` to
// `Result`, since it is used all over the place.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub(crate) struct NomContext(String);

impl NomContext {
    pub fn section(value: &str) -> Self {
        Self(format!("the `{value}` section encountered"))
    }

    pub fn _other(value: &str) -> Self {
        Self(value.into())
    }
}

impl Display for NomContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

type Input<'a> = &'a [u8];
pub(crate) type NomError<'a> = GenericErrorTree<Input<'a>, Input<'a>, NomContext, ()>;
pub(crate) type NomResult<'a, T> = nom::IResult<Input<'a>, T, NomError<'a>>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(code(rashen::io_error))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(
        // TODO(Unavailable): The `code` url could redirect to the format spec in the README.md
        code(rashen::invalid_format),
        help("Make sure that your `packfile.dat` is from the version `1.0.6`.")
    )]
    InvalidFormat(
        #[from]
        #[source_code]
        // NOTE: miette needs at least `1` label to be able to display the source code.
        InvalidFormatError,
    ),
}

impl From<NomError<'_>> for Error {
    fn from(value: NomError<'_>) -> Self {
        Error::from(InvalidFormatError::from(value))
    }
}

#[derive(Error, Diagnostic, Debug)]
pub struct InvalidFormatError {
    input: Vec<u8>,
    #[source_code]
    source_code: String,
}

impl From<NomError<'_>> for InvalidFormatError {
    fn from(_value: NomError<'_>) -> Self {
        todo!()
    }
}

impl SourceCode for InvalidFormatError {
    fn read_span<'a>(
        &'a self,
        _: &SourceSpan,
        _: usize,
        _: usize,
    ) -> std::result::Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        todo!()
    }
}

impl Display for InvalidFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.input))
    }
}
