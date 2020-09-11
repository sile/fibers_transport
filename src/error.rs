use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt, TrackableError};

/// This crate specific [`Error`] type.
///
/// [`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html
#[derive(Debug, Clone, TrackableError)]
pub struct Error(TrackableError<ErrorKind>);
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        ErrorKind::IoError.cause(f).into()
    }
}
impl From<bytecodec::Error> for Error {
    fn from(f: bytecodec::Error) -> Self {
        let original_error_kind = *f.kind();
        if f.concrete_cause::<std::io::Error>().is_some() {
            track!(ErrorKind::IoError.takes_over(f); original_error_kind).into()
        } else {
            track!(ErrorKind::CodecError.takes_over(f); original_error_kind).into()
        }
    }
}

/// Possible error kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Encoding/decoding error.
    CodecError,

    /// I/O error.
    IoError,

    /// Input is invalid.
    InvalidInput,

    /// Other error.
    Other,
}
impl TrackableErrorKind for ErrorKind {}
