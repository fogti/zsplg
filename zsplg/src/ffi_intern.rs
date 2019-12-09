use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Cast,
    Encoding,
}

impl From<IoError> for Error {
    fn from(x: IoError) -> Error {
        Error::Io(x)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => fmt::Display::fmt(e, f),
            Error::Cast => write!(f, "wrapper cast failed"),
            Error::Encoding => write!(
                f,
                "byte sequence is not representable in the platform encoding"
            ),
        }
    }
}
