pub mod ffi_extern;
pub mod ffi_intern;

use libloading::Symbol;
use std::{
    ffi::{CStr, OsStr},
    sync::Arc,
};
use zsplg_core::Wrapper as FFIWrapper;

pub struct Plugin {
    user_data: FFIWrapper,
    modname: Vec<u8>,
    dlh: libloading::Library,
}

pub struct Handle {
    user_data: FFIWrapper,
    parent: Arc<Plugin>,
}

impl Drop for Plugin {
    fn drop(&mut self) {
        self.user_data.call_dtor();
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.user_data.call_dtor();
    }
}

impl Plugin {
    fn get_fn<T>(&self, prefix: &[u8], name: &[u8]) -> Result<Symbol<'_, T>, std::io::Error> {
        let mut real_name: Vec<u8> =
            Vec::with_capacity(self.modname.len() + prefix.len() + name.len() + 2);
        real_name.extend(self.modname.iter().copied());
        real_name.push(b'_');
        real_name.extend(prefix.iter().copied());
        real_name.extend(name.iter().copied());
        real_name.push(b'\0');
        unsafe { self.dlh.get(&real_name[..]) }
    }

    pub fn new(file: Option<&OsStr>, modname: &CStr) -> Result<Plugin, std::io::Error> {
        let mut ret = Plugin {
            user_data: FFIWrapper::null(),
            modname: modname.to_bytes().to_owned(),
            dlh: match file {
                Some(file) => libloading::Library::new(file)?,
                None => {
                    #[cfg(not(unix))]
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "selfexe-referential plugins aren't supported on this platform",
                    ));
                    #[cfg(unix)]
                    libloading::os::unix::Library::this().into()
                }
            },
        };
        // call initialization function
        ret.user_data = (ret.get_fn::<extern "C" fn() -> FFIWrapper>(b"", b"init")?)();
        Ok(ret)
    }

    pub fn create_handle(this: &Arc<Self>, args: &[FFIWrapper]) -> Result<Handle, std::io::Error> {
        let hcfn: Symbol<extern "C" fn(*const FFIWrapper, usize, *const FFIWrapper) -> FFIWrapper> =
            this.get_fn(b"", b"hcreate")?;

        Ok(Handle {
            user_data: hcfn(&this.user_data, args.len(), args.as_ptr()),
            parent: Arc::clone(this),
        })
    }

    fn call_intern(
        &self,
        hsel: Option<&FFIWrapper>,
        fname: &CStr,
        args: &[FFIWrapper],
    ) -> Result<FFIWrapper, std::io::Error> {
        let xfn: Symbol<extern "C" fn(*const FFIWrapper, usize, *const FFIWrapper) -> FFIWrapper> =
            self.get_fn(if hsel.is_some() { b"h_" } else { b"_" }, fname.to_bytes())?;

        Ok(xfn(
            hsel.unwrap_or(&self.user_data),
            args.len(),
            args.as_ptr(),
        ))
    }
}

pub trait RTMultiFn {
    fn call(&self, fname: &CStr, args: &[FFIWrapper]) -> Result<FFIWrapper, std::io::Error>;
}

impl RTMultiFn for Plugin {
    #[inline]
    fn call(&self, fname: &CStr, args: &[FFIWrapper]) -> Result<FFIWrapper, std::io::Error> {
        self.call_intern(None, fname, args)
    }
}

impl RTMultiFn for Handle {
    #[inline]
    fn call(&self, fname: &CStr, args: &[FFIWrapper]) -> Result<FFIWrapper, std::io::Error> {
        self.parent.call_intern(Some(&self.user_data), fname, args)
    }
}
