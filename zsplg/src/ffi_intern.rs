use std::fmt;
use std::io::Error as IoError;
use zsplg_core::Wrapper;

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

#[repr(C)]
pub enum Result<T> {
    Ok(T),
    Err(Wrapper),
}

impl<T, E> From<::std::result::Result<T, E>> for Result<T>
where
    E: Into<Error>,
{
    fn from(x: ::std::result::Result<T, E>) -> Result<T> {
        match x {
            ::std::result::Result::Ok(y) => Result::Ok(y),
            ::std::result::Result::Err(y) => {
                Result::Err(unsafe { Wrapper::new::<Error>(y.into()) })
            }
        }
    }
}

pub fn wrap_to_c<T, E>(x: ::std::result::Result<T, E>) -> Result<Wrapper>
where
    T: 'static,
    E: Into<Error>,
{
    x.map(|y| unsafe { Wrapper::new(y) }).into()
}
