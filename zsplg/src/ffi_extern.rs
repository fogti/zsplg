/// === C FFI ===
use std::{
    any::Any,
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    sync::Arc,
};

use crate::ffi_intern::Error as FFIError;
use crate::{Handle, Plugin};

//type ResultWrap = crate::ffi_intern::Result<FFIWrapper>;

/// real FFI wrapper
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Object {
    data: *const c_void,
    meta: usize,
}

pub struct FFIResult {
    data: Object,
    // optimize padding
    is_success: bool,
}

pub(crate) type RealOptObj = Option<Arc<dyn Any + Send + Sync>>;

impl From<RealOptObj> for Object {
    fn from(x: RealOptObj) -> Object {
        match x {
            Some(y) => unsafe {
                let [data, meta] = zsplg_core::fatptr::decomp(Arc::into_raw(y));
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
                Arc::from_raw(zsplg_core::fatptr::recomp([
                    std::mem::transmute::<_, _>(self.data),
                    self.meta,
                ]))
            })
        } else {
            None
        }
    }
}

fn wrapres<T, F>(x: Result<T, T>, f: F) -> FFIResult
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

fn partial_wrap<T>(x: T) -> RealOptObj
where
    T: Send + Sync + 'static,
{
    Some(Arc::new(x) as Arc<dyn Any + Send + Sync + 'static>)
}

fn full_wrapres<T, E>(x: Result<T, E>) -> FFIResult
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

#[no_mangle]
pub unsafe extern "C" fn zsplg_open(file: *const c_char, modname: *const c_char) -> FFIResult {
    let file = if file.is_null() {
        None
    } else {
        match os_str_bytes::OsStrBytes::from_bytes(CStr::from_ptr(file).to_bytes()) {
            Ok(x) => Some(x),
            Err(_) => {
                return Err(Into::<Object>::into(Some(
                    Arc::new(FFIError::Encoding) as Arc<dyn Any + Send + Sync>
                )))
                .into()
            }
        }
    };
    full_wrapres(Plugin::new(
        file.as_ref().map(std::ops::Deref::deref),
        CStr::from_ptr(modname),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn zsplg_h_create(
    parent: Object,
    argc: usize,
    argv: *const Object,
) -> FFIResult {
    let parent: RealOptObj = parent.into();
    let inner = || {
        if let Some(parent) = parent {
            // each branch should run 'mem::forget', because the caller owns the 'parent'
            match parent.downcast::<Plugin>() {
                Ok(parent) => {
                    let ret =
                        Plugin::create_handle(&parent, std::slice::from_raw_parts(argv, argc));
                    std::mem::forget(parent);
                    return ret.map_err(|e| e.into());
                }
                Err(parent) => std::mem::forget(parent),
            }
        }
        Err(FFIError::Cast)
    };

    full_wrapres(inner())
}

#[no_mangle]
pub unsafe extern "C" fn zsplg_call(
    obj: Object,
    fname: *const c_char,
    argc: usize,
    argv: *const Object,
) -> FFIResult {
    let obj: RealOptObj = obj.into();
    let obj = obj.unwrap();

    let inner = |obj_: &Arc<dyn Any + Send + Sync + 'static>| {
        let rtmf: &dyn crate::RTMultiFn = if let Some(handle) = obj_.downcast_ref::<Handle>() {
            handle
        } else if let Some(plg) = obj_.downcast_ref::<Plugin>() {
            plg
        } else {
            return Err(FFIError::Cast);
        };

        rtmf.call(
            CStr::from_ptr(fname),
            std::slice::from_raw_parts(argv, argc),
        )
        .map_err(Into::<FFIError>::into)
    };

    let ret = inner(&obj);

    // the caller owns the `Arc`
    std::mem::forget(obj);

    wrapres(ret.map_err(|x| partial_wrap(x).into()), Into::into)
}

/// This function converts an error to a wrapped string
/// Consumes the error.
#[no_mangle]
pub extern "C" fn zsplg_error_to_str(e: Object) -> Object {
    let obj: RealOptObj = e.into();
    partial_wrap(
        obj.and_then(|obj| obj.downcast_ref::<FFIError>().map(|e| format!("{}", e)))
            .unwrap_or_else(String::new),
    )
    .into()
}

#[no_mangle]
pub extern "C" fn zsplg_is_null(w: Object) -> bool {
    let obj: RealOptObj = w.into();
    let res = obj.is_none();
    std::mem::forget(obj);
    res
}

/// Clones the given string into a newly allocated object on the heap
#[no_mangle]
pub unsafe extern "C" fn zsplg_new_str(x: *const c_char) -> Object {
    if !x.is_null() {
        partial_wrap(CString::new(CStr::from_ptr(x).to_bytes().to_owned()))
    } else {
        None
    }
    .into()
}

/// Needed to access the error string returned by `zsplg_error_to_str` or `zsplg_new_str`
#[no_mangle]
pub unsafe extern "C" fn zsplg_get_str(w: Object) -> *const c_char {
    let obj: RealOptObj = w.into();
    if let Some(x) = obj.and_then(|obj| obj.downcast_ref::<CString>().map(|x| x.as_ptr())) {
        x
    } else {
        std::ptr::null()
    }
}

#[no_mangle]
pub unsafe extern "C" fn zsplg_destroy(w: Object) -> bool {
    let obj: RealOptObj = w.into();
    obj.is_some()
}
