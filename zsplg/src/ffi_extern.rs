/// === C FFI ===
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};
use zsplg_core::{bool_to_c, c_bool, Wrapper};

use crate::ffi_intern::{wrap_to_c, Error as FFIError};
use crate::Plugin;

#[no_mangle]
pub extern "C" fn zsplg_open(file: *const c_char, modname: *const c_char) -> crate::ffi_intern::Result<Wrapper> {
    let file = if file.is_null() {
        None
    } else {
        match os_str_bytes::OsStrBytes::from_bytes(unsafe { CStr::from_ptr(file).to_bytes() }) {
            Ok(x) => Some(x),
            Err(_) => return wrap_to_c::<Wrapper, _>(Err(FFIError::Encoding)),
        }
    };
    wrap_to_c(Plugin::new(
        file.as_ref().map(std::ops::Deref::deref),
        unsafe { CStr::from_ptr(modname) },
    ))
}

/// This function converts an error to a wrapped string
#[no_mangle]
pub extern "C" fn zsplg_error_to_str(e: &Wrapper) -> Wrapper {
    if let Some(e) = e.try_cast_sized::<FFIError>() {
        unsafe { Wrapper::new(CString::new(format!("{}", e))) }
    } else {
        Wrapper::null()
    }
}

#[no_mangle]
pub extern "C" fn zsplg_is_null(w: &Wrapper) -> c_bool {
    bool_to_c(w == &Wrapper::null())
}

/// Clones the given string into a newly allocated object on the heap
#[no_mangle]
pub extern "C" fn zsplg_new_str(x: *const c_char) -> Wrapper {
    if !x.is_null() {
        unsafe { Wrapper::new(CString::new(CStr::from_ptr(x).to_bytes().to_owned())) }
    } else {
        Wrapper::null()
    }
}

/// Needed to access the error string returned by `zsplg_error_to_str` or `zsplg_new_str`
#[no_mangle]
pub extern "C" fn zsplg_get_str(w: *const Wrapper) -> *const c_char {
    if let Some(w) = unsafe { w.as_ref() } {
        if let Some(x) = Wrapper::try_cast_sized::<CString>(w) {
            return x.as_ptr();
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn zsplg_destroy(wrap: *mut Wrapper) -> c_bool {
    bool_to_c(unsafe { wrap.as_mut().map(Wrapper::call_dtor) == Some(true) })
}
