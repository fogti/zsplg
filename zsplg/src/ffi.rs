/// === C FFI ===
use std::{
    any::Any,
    ffi::{CStr, CString},
    os::raw::c_char,
    sync::Arc,
};

use crate::{Handle, Plugin};
use try_block::try_block;
use zsplg_core::{wrap, wrapres, Error as FFIError, FFIResult, Object, RealOptObj};

#[no_mangle]
pub unsafe extern "C" fn zsplg_open(file: *const c_char, modname: *const c_char) -> FFIResult {
    let file = if file.is_null() {
        None
    } else {
        match os_str_bytes::OsStrBytes::from_bytes(CStr::from_ptr(file).to_bytes()) {
            Ok(x) => Some(x),
            Err(_) => return Err(wrap(FFIError::Encoding)).into(),
        }
    };
    wrapres(Plugin::new(
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

    ret.map_err(wrap).into()
}

/// This function converts an error to a wrapped string
/// Consumes the error.
#[no_mangle]
pub extern "C" fn zsplg_error_to_str(e: Object) -> Object {
    let obj: RealOptObj = e.into();
    wrap(
        obj.and_then(|obj| obj.downcast_ref::<FFIError>().map(|e| format!("{}", e)))
            .unwrap_or_else(String::new),
    )
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
        wrap(CString::new(CStr::from_ptr(x).to_bytes().to_owned()))
    } else {
        None.into()
    }
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
#[inline]
pub unsafe extern "C" fn zsplg_destroy(w: Object) -> bool {
    Into::<RealOptObj>::into(w).is_some()
}
