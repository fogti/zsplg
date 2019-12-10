use std::{
    any::Any,
    ffi::c_void,
    sync::Arc,
    fmt,
    io::Error as IoError,
};

mod fatptr;

/// === C FFI ===
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

/// real FFI wrapper
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Object {
    pub data: *const c_void,
    pub meta: usize,
}

pub struct FFIResult {
    pub data: Object,
    // optimize padding
    pub is_success: bool,
}

pub type RealOptObj = Option<Arc<dyn Any + Send + Sync>>;

impl From<RealOptObj> for Object {
    fn from(x: RealOptObj) -> Object {
        match x {
            Some(y) => unsafe {
                let [data, meta] = crate::fatptr::decomp(Arc::into_raw(y));
                Object {
                    data: std::mem::transmute::<_, _>(data),
                    meta,
                }
            },
            None => Object {
                data: std::ptr::null(),
                meta: 0,
            },
        }
    }
}

impl Into<RealOptObj> for Object {
    fn into(self) -> RealOptObj {
        if !self.data.is_null() && self.meta != 0 {
            Some(unsafe {
                Arc::from_raw(crate::fatptr::recomp([
                    std::mem::transmute::<_, _>(self.data),
                    self.meta,
                ]))
            })
        } else {
            None
        }
    }
}

pub fn wrapres<T, F>(x: Result<T, T>, f: F) -> FFIResult
where
    F: FnOnce(T) -> Object,
{
    let is_success = x.is_ok();
    FFIResult {
        data: match x {
            Ok(y) | Err(y) => f(y),
        },
        is_success,
    }
}

pub fn partial_wrap<T>(x: T) -> RealOptObj
where
    T: Send + Sync + 'static,
{
    Some(Arc::new(x) as Arc<dyn Any + Send + Sync + 'static>)
}

pub fn full_wrapres<T, E>(x: Result<T, E>) -> FFIResult
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    wrapres(
        x.map(partial_wrap::<T>).map_err(partial_wrap::<E>),
        Into::into,
    )
}

impl From<Result<Object, Object>> for FFIResult {
    fn from(x: Result<Object, Object>) -> FFIResult {
        wrapres(x, std::convert::identity)
    }
}

impl From<Result<RealOptObj, RealOptObj>> for FFIResult {
    fn from(x: Result<RealOptObj, RealOptObj>) -> FFIResult {
        wrapres(x, Into::into)
    }
}
