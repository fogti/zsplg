/**
  expected plugin functions
Plugin * MODNAME_init()
void * MODNAME__XFN(void * data, size_t argc, char *argv[])
void * MODNAME_h_XFN(void * handle, size_t argc, char *argv[])
 */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Gdsa {
    pub data: *mut libc::c_void,
    pub len: usize,
    pub destroy: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> bool>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Plugin {
    pub data: Gdsa,
    pub fn_h_create: Option<
        unsafe extern "C" fn(
            _: *mut libc::c_void,
            _: usize,
            _: *mut *const libc::c_char,
        ) -> Gdsa,
    >,
}

pub const ZS_GDSA_NULL: Gdsa = Gdsa {
    data: std::ptr::null_mut(),
    len: 0,
    destroy: None,
};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Status {
    OK = 0,
    DLOPN = 1,
    DLCLOS = 2,
    DLSYM = 3,
    PLG = 4,
}

impl Status {
    #[inline]
    pub fn is_ok(self) -> bool {
        self == Status::OK
    }
    #[inline]
    pub fn is_err(self) -> bool {
        !self.is_ok()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Handle {
    plugin: Plugin,
    pub st: Status,
    mnlen: crate::stddef_h::size_t,
    modname: *const libc::c_char,
    pub error_str: *const libc::c_char,
    dlh: *mut libc::c_void,
    have_alloc: bool,
}

fn setstdl(handle: &mut Handle, st: Status) {
    handle.st = st;
    handle.error_str = unsafe { crate::stdlib::dlerror() };
}

macro_rules! zspe {
    ($x:ident) => { Status::$x }
}

#[link_name = "zsplg_open"]
pub unsafe extern "C" fn open(
    file: *const libc::c_char,
    modname: *const libc::c_char,
    do_strcpy: bool,
) -> Handle {
    /* load plugin with dlopen */
    let mut ret = Handle {
        plugin: Plugin {
            data: Gdsa {
                data: std::ptr::null_mut(),
                len: 0,
                destroy: None,
            },
            fn_h_create: None,
        },
        st: Status::OK,
        mnlen: crate::stdlib::strlen(modname),
        modname: std::ptr::null(),
        error_str: std::ptr::null(),
        dlh: if !file.is_null() {
            crate::stdlib::dlopen(file, crate::stdlib::RTLD_LAZY | crate::stdlib::RTLD_LOCAL)
        } else {
            crate::stdlib::RTLD_DEFAULT as *mut libc::c_void
        },
        have_alloc: do_strcpy,
    };
    if !file.is_null() && ret.dlh.is_null() {
        setstdl(&mut ret, zspe!(DLOPN));
        return ret;
    }
    /* init_fn_name = "init_" + modname + "\0" */
    let vla = ret.mnlen.wrapping_add(6i32 as libc::c_ulong) as usize;
    let mut init_fn_name: Vec<libc::c_char> = ::std::vec::from_elem(0, vla);
    let mut tmp: *mut libc::c_char = init_fn_name.as_mut_ptr();
    crate::xcpy::llzs_strixcpy(
        &mut tmp as *mut *mut libc::c_char,
        b"init_\x00" as *const u8 as *const libc::c_char,
        5i32 as crate::stddef_h::size_t,
    );
    crate::xcpy::llzs_strixcpy(&mut tmp as *mut *mut libc::c_char, modname, ret.mnlen);
    let init_fn: Option<unsafe extern "C" fn() -> *mut Plugin> = ::std::mem::transmute::<
        *mut libc::c_void,
        Option<unsafe extern "C" fn() -> *mut Plugin>,
    >(crate::stdlib::dlsym(
        ret.dlh,
        init_fn_name.as_mut_ptr(),
    ));
    /* initialize plugin */
    if init_fn.is_some() {
        ret.plugin = *::std::mem::transmute::<_, fn() -> *mut Plugin>(
            init_fn.expect("non-null function pointer"),
        )();
        ret.modname = if do_strcpy as libc::c_int != 0 {
            crate::xcpy::llzs_strxdup(modname, ret.mnlen) as *const libc::c_char
        } else {
            modname
        }
    } else {
        setstdl(&mut ret, zspe!(DLOPN));
        /* cleanup dlh */
        if !file.is_null() {
            crate::stdlib::dlclose(ret.dlh);
            ret.dlh = std::ptr::null_mut()
        }
    }
    return ret;
}

#[link_name = "zsplg_destroy"]
pub unsafe extern "C" fn destroy(gdsa: *mut Gdsa) -> bool {
    // gdsa->destroy != 0 --> successful
    let ret: bool = !gdsa.is_null()
        && ((*gdsa).destroy.is_none()
            || (*gdsa).destroy.expect("non-null function pointer")((*gdsa).data) as libc::c_int
                != 0);
    if ret {
        (*gdsa).destroy = None;
        (*gdsa).data = std::ptr::null_mut();
        (*gdsa).len = 0;
    }
    return ret;
}

#[link_name = "zsplg_close"]
pub unsafe extern "C" fn close(handle: *mut Handle) -> bool {
    if handle.is_null() {
        return false;
    }
    let mut st = (*handle).st;
    if st == zspe!(DLOPN) {
        return false;
    }
    let plgptr: *mut Plugin = &mut (*handle).plugin;
    if !destroy(&mut (*plgptr).data) {
        st = zspe!(PLG)
    }
    /* unload plugin */
    if !(*handle).dlh.is_null() {
        if crate::stdlib::dlclose((*handle).dlh) == 0 {
            (*handle).dlh = std::ptr::null_mut()
        } else {
            if st == Status::OK {
                st = zspe!(DLCLOS)
            }
            (*handle).error_str = crate::stdlib::dlerror()
        }
    }
    if (*handle).have_alloc {
        crate::stdlib::free((*handle).modname as *mut libc::c_void);
        (*handle).modname = std::ptr::null()
    }
    (*handle).st = st;
    return st.is_ok();
}

#[link_name = "zsplg_h_create"]
pub unsafe extern "C" fn h_create(
    base: *const Handle,
    argc: usize,
    argv: *mut *const libc::c_char,
) -> Gdsa {
    let plgptr = &(*base).plugin;
    return plgptr.fn_h_create.expect("non-null function pointer")(plgptr.data.data, argc, argv);
}

fn upd_errstr(handle: &mut Handle, st: Status) {
    let tmp = unsafe { crate::stdlib::dlerror() };
    if !tmp.is_null() {
        handle.error_str = tmp;
        if st.is_err() {
            handle.st = st;
        }
    }
}

#[link_name = "zsplg_call_h"]
pub unsafe extern "C" fn call_h(
    handle: *mut Handle,
    h_id: *mut libc::c_void,
    fn_0: *const libc::c_char,
    argc: usize,
    argv: *mut *const libc::c_char,
) -> Gdsa {
    /* handle error conditions */
    if handle.is_null() || fn_0.is_null() || (*handle).st == zspe!(DLOPN) {
        return ZS_GDSA_NULL;
    }
    /* construct function name */
    let fnlen = crate::stdlib::strlen(fn_0);
    let vla = ((*handle).mnlen + fnlen + 3 + (if !h_id.is_null() { 1 } else { 0 })) as usize;
    let mut xfn_name: Vec<libc::c_char> = ::std::vec::from_elem(0, vla);
    let mut xnp: *mut libc::c_char =
        crate::xcpy::llzs_strxcpy(xfn_name.as_mut_ptr(), (*handle).modname, (*handle).mnlen);
    *xnp = '_' as libc::c_char;
    xnp = xnp.offset(1);
    if !h_id.is_null() {
        *xnp = 'h' as libc::c_char;
        xnp = xnp.offset(1);
    }
    crate::xcpy::llzs_strxcpy(
        crate::stdlib::stpcpy(xnp, b"_\x00" as *const u8 as *const libc::c_char),
        fn_0,
        fnlen,
    );
    /* get function addr */
    upd_errstr(&mut *handle, Status::OK);
    let xfn_ptr: Option<
        unsafe extern "C" fn(
            _: *mut libc::c_void,
            _: usize,
            _: *const *const libc::c_char,
        ) -> Gdsa,
    > = ::std::mem::transmute::<
        *mut libc::c_void,
        Option<
            unsafe extern "C" fn(
                _: *mut libc::c_void,
                _: usize,
                _: *const *const libc::c_char,
            ) -> Gdsa,
        >,
    >(crate::stdlib::dlsym((*handle).dlh, xfn_name.as_mut_ptr()));
    if xfn_ptr.is_none() {
        upd_errstr(&mut *handle, zspe!(DLSYM));
        return ZS_GDSA_NULL;
    }
    /* call function */
    return xfn_ptr.expect("non-null function pointer")(
        if !h_id.is_null() {
            h_id
        } else {
            (*handle).plugin.data.data
        },
        argc,
        argv as *const *const libc::c_char,
    );
}
