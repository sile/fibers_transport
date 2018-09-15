use bytecodec;
use std;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt, TrackableError};

#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        ErrorKind::IoError.cause(f).into()
    }
}
impl From<bytecodec::Error> for Error {
    fn from(f: bytecodec::Error) -> Self {
        let original_error_kind = *f.kind();
        track!(ErrorKind::CodecError.takes_over(f); original_error_kind).into()
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    CodecError,
    IoError,
    Other,
}
impl TrackableErrorKind for ErrorKind {}
