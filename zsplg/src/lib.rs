pub mod ffi_extern;
pub mod ffi_intern;
use ffi_extern::{Object, RealOptObj};

use libloading::Symbol;
use std::{
    any::Any,
    ffi::{CStr, OsStr},
    sync::Arc,
};

pub struct Plugin {
    user_data: Option<Arc<dyn Any + Send + Sync>>,
    modname: Vec<u8>,
    dlh: libloading::Library,
}

pub struct Handle {
    user_data: Arc<dyn Any + Send + Sync>,
    parent: Arc<Plugin>,
}

impl Drop for Plugin {
    fn drop(&mut self) {
        // we must free the user_data before the 'parent' object is destroyed
        self.user_data.take();
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
            user_data: None,
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
        ret.user_data =
            (ret.get_fn::<extern "C" fn() -> ffi_extern::Object>(b"", b"init")?)().into();
        Ok(ret)
    }

    pub fn create_handle(this: &Arc<Self>, args: &[Object]) -> Result<Handle, std::io::Error> {
        let hcfn: Symbol<extern "C" fn(Object, usize, *const Object) -> Object> =
            this.get_fn(b"", b"hcreate")?;

        // ref
        let xsel: Object = Some(this.user_data.as_ref().unwrap().clone()).into();
        let user_data: RealOptObj = hcfn(xsel, args.len(), args.as_ptr()).into();

        let ret = Ok(Handle {
            user_data: user_data.unwrap(),
            parent: Arc::clone(this),
        });

        // unref
        let _: RealOptObj = xsel.into();

        ret
    }

    fn call_intern(
        &self,
        hsel: Option<&Arc<dyn Any + Send + Sync>>,
        fname: &CStr,
        args: &[Object],
    ) -> Result<Object, std::io::Error> {
        let xfn: Symbol<extern "C" fn(Object, usize, *const Object) -> Object> =
            self.get_fn(if hsel.is_some() { b"h_" } else { b"_" }, fname.to_bytes())?;

        // ref
        let xsel: RealOptObj = Some(
            hsel.unwrap_or_else(|| self.user_data.as_ref().unwrap())
                .clone(),
        );
        let xsel: Object = xsel.into();

        let ret = Ok(xfn(xsel, args.len(), args.as_ptr()));

        // unref
        let _: RealOptObj = xsel.into();

        ret
    }
}

pub trait RTMultiFn {
    fn call(&self, fname: &CStr, args: &[Object]) -> Result<Object, std::io::Error>;
}

impl RTMultiFn for Plugin {
    #[inline]
    fn call(&self, fname: &CStr, args: &[Object]) -> Result<Object, std::io::Error> {
        self.call_intern(None, fname, args)
    }
}

impl RTMultiFn for Handle {
    #[inline]
    fn call(&self, fname: &CStr, args: &[Object]) -> Result<Object, std::io::Error> {
        self.parent.call_intern(Some(&self.user_data), fname, args)
    }
}
