//! === C FFI ===
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

use crate::{Handle, Plugin};
use try_block::try_block;
use zsplg_core::{wrap, wrapres, Error as FFIError, FFIResult, Object, RealOptObj};

#[no_mangle]
pub unsafe extern "C" fn zsplg_open(file: *const c_char, modname: *const c_char) -> FFIResult {
    wrapres(Plugin::new(
        if file.is_null() {
            None
        } else {
            match os_str_bytes::OsStrBytes::from_bytes(CStr::from_ptr(file).to_bytes()) {
                Ok(x) => Some(x),
                Err(_) => return Err(wrap(FFIError::Encoding)).into(),
            }
        }
        .as_ref()
        .map(std::ops::Deref::deref),
        CStr::from_ptr(modname),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn zsplg_h_create(
    parent: Object,
    argc: usize,
    argv: *const Object,
) -> FFIResult {
    wrapres(try_block! {
        if let Some(parent) = Into::<RealOptObj>::into(parent) {
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
    })
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

    let ret = try_block! {
        let rtmf: &dyn crate::RTMultiFn = if let Some(handle) = obj.downcast_ref::<Handle>() {
            handle
        } else if let Some(plg) = obj.downcast_ref::<Plugin>() {
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

    // the caller owns the `Arc`
    std::mem::forget(obj);

    ret.map_err(wrap).into()
}

/// This function converts an error to a wrapped string
/// Consumes the error.
#[no_mangle]
pub unsafe extern "C" fn zsplg_error_to_str(e: Object) -> Object {
    let obj: RealOptObj = e.into();
    wrap(
        obj.and_then(|obj| obj.downcast_ref::<FFIError>().map(|e| format!("{}", e)))
            .unwrap_or_else(String::new),
    )
}

#[no_mangle]
pub extern "C" fn zsplg_is_null(w: Object) -> bool {
    // we don't need to reconstruct the Arc
    w.is_null()
}

/// Clones the given string into a newly allocated object on the heap
#[no_mangle]
pub unsafe extern "C" fn zsplg_new_str(x: *const c_char) -> Object {
    if !x.is_null() {
        wrap(CString::new(CStr::from_ptr(x).to_bytes().to_owned()).unwrap())
    } else {
        None.into()
    }
}

/// Needed to access the error string returned by `zsplg_error_to_str` or `zsplg_new_str`
///
/// # Safety
/// The returned pointer should never outlive the given input object.
#[no_mangle]
pub unsafe extern "C" fn zsplg_get_str(w: Object) -> *const c_char {
    Into::<RealOptObj>::into(w)
        .and_then(|obj| {
            let ret = obj.downcast_ref::<CString>().map(|x| x.as_ptr());
            // the caller owns the `Arc`
            std::mem::forget(obj);
            ret
        })
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
#[inline]
pub unsafe extern "C" fn zsplg_destroy(w: Object) -> bool {
    Into::<RealOptObj>::into(w).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings() {
        let x = CStr::from_bytes_with_nul(b"Test String!\0").unwrap();
        let obj = unsafe { zsplg_new_str(x.as_ptr()) };
        {
            let ptr2o = unsafe { zsplg_get_str(obj) };
            // make sure repeated calls to `zsplg_get_str` don't invalidate the string
            let _ = unsafe { zsplg_get_str(obj) };
            let ptr2o = unsafe { CStr::from_ptr(ptr2o) };
            assert_eq!(x, ptr2o);
        }
        unsafe { zsplg_destroy(obj) };
    }
}
