use std::{any::Any, ffi::c_void, fmt, io::Error as IoError, sync::Arc};

mod fatptr;

/// === C FFI ===
#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Cast,
    Encoding,
}

impl From<IoError> for Error {
    #[inline(always)]
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

impl Object {
    pub fn is_null(&self) -> bool {
        self.data.is_null() && self.meta == 0
    }
}

impl From<RealOptObj> for Object {
    fn from(x: RealOptObj) -> Object {
        match x {
            Some(y) => unsafe {
                let [data, meta] = crate::fatptr::decomp(Arc::into_raw(y));
                Object {
                    data: data as *const c_void,
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
        if !self.is_null() {
            Some(unsafe { Arc::from_raw(crate::fatptr::recomp([self.data as usize, self.meta])) })
        } else {
            None
        }
    }
}

impl From<Result<Object, Object>> for FFIResult {
    #[inline]
    fn from(x: Result<Object, Object>) -> FFIResult {
        let is_success = x.is_ok();
        FFIResult {
            data: match x {
                Ok(y) | Err(y) => y,
            },
            is_success,
        }
    }
}

#[inline]
pub fn wrap<T>(x: T) -> Object
where
    T: Send + Sync + 'static,
{
    Some(Arc::new(x) as Arc<dyn Any + Send + Sync + 'static>).into()
}

#[inline]
pub fn wrapres<T, E>(x: Result<T, E>) -> FFIResult
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    x.map(wrap::<T>).map_err(wrap::<E>).into()
}
