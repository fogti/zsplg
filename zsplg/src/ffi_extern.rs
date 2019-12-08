/// === C FFI ===
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    sync::Arc,
};
use zsplg_core::{bool_to_c, c_bool, Wrapper};

use crate::ffi_intern::{wrap_to_c, Error as FFIError};
use crate::{Handle, Plugin};

type ResultWrap = crate::ffi_intern::Result<Wrapper>;

#[no_mangle]
pub extern "C" fn zsplg_open(file: *const c_char, modname: *const c_char) -> ResultWrap {
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

#[no_mangle]
pub unsafe extern "C" fn zsplg_h_create(
    parent: &Wrapper,
    argc: usize,
    argv: *const Wrapper,
) -> ResultWrap {
    let args = std::slice::from_raw_parts(argv, argc);
    if let Some(parent) = parent.try_cast_raw::<zsplg_core::WrapSized<Plugin>>() {
        // we use transmute to get rid of the 'WrapSized' new-type
        wrap_to_c(Plugin::create_handle(
            &std::mem::transmute::<_, Arc<Plugin>>(parent),
            args,
        ))
    } else {
        crate::ffi_intern::Result::Err(Wrapper::new(FFIError::Cast))
    }
}

#[no_mangle]
pub unsafe extern "C" fn zsplg_call(
    obj: &Wrapper,
    fname: *const c_char,
    argc: usize,
    argv: *const Wrapper,
) -> ResultWrap {
    let rtmf: Arc<dyn crate::RTMultiFn> =
        if let Some(handle) = obj.try_cast_raw::<zsplg_core::WrapSized<Handle>>() {
            std::mem::transmute::<_, Arc<Handle>>(handle)
        } else if let Some(plg) = obj.try_cast_raw::<zsplg_core::WrapSized<Plugin>>() {
            std::mem::transmute::<_, Arc<Plugin>>(plg)
        } else {
            return crate::ffi_intern::Result::Err(Wrapper::new(FFIError::Cast));
        };
    let fname = CStr::from_ptr(fname);
    let args = std::slice::from_raw_parts(argv, argc);
    wrap_to_c(rtmf.call(fname, args))
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
