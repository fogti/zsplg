use std::ffi::{CStr, CString, OsStr};

pub mod ffi_extern;
pub mod ffi_intern;

use zsplg_core::Wrapper as FFIWrapper;

pub struct Plugin {
    user_data: FFIWrapper,
    modname: CString,
    dlh: libloading::Library,
}

impl Plugin {
    pub fn new(file: Option<&OsStr>, modname: &CStr) -> Result<Plugin, std::io::Error> {
        let mut ret = Plugin {
            user_data: FFIWrapper::null(),
            modname: modname.to_owned(),
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
        let mut init_name: Vec<u8> = ret.modname.as_bytes().to_owned();
        init_name.extend(b"_init\0");
        let init: libloading::Symbol<extern "C" fn() -> FFIWrapper> =
            unsafe { ret.dlh.get(&init_name[..]) }?;
        ret.user_data = init();
        Ok(ret)
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        self.user_data.call_dtor();
    }
}
