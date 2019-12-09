use super::{Wrapped, WrapperInner};
use dyn_sized::DynSized;

mod private {
    use crate::WrapMeta;

    pub trait MetaWrapped: Copy + Sized {
        fn wrap(self) -> WrapMeta;
        fn try_unwrap(x: WrapMeta) -> Option<Self>;
    }

    impl MetaWrapped for usize {
        fn wrap(self) -> WrapMeta {
            WrapMeta::Length(self)
        }

        fn try_unwrap(x: WrapMeta) -> Option<Self> {
            if let WrapMeta::Length(y) = x {
                Some(y)
            } else {
                None
            }
        }
    }

    impl MetaWrapped for *mut () {
        fn wrap(self) -> WrapMeta {
            WrapMeta::TraitObject(self as *mut core::ffi::c_void)
        }

        fn try_unwrap(x: WrapMeta) -> Option<Self> {
            if let WrapMeta::TraitObject(y) = x {
                Some(y as *mut ())
            } else {
                None
            }
        }
    }
}

unsafe impl<T> Wrapped for T
where
    T: ?Sized + dyn_sized::DynSized + 'static,
    T::Meta: private::MetaWrapped,
{
    fn wrap(x: *const Self) -> WrapperInner {
        let (meta, ptr) = DynSized::disassemble(x);
        WrapperInner {
            data: ptr as *const super::c_void,
            meta: private::MetaWrapped::wrap(meta),
        }
    }

    fn as_ptr(x: &WrapperInner) -> *const Self {
        if let Some(meta) = <T::Meta as private::MetaWrapped>::try_unwrap(x.meta) {
            DynSized::assemble(meta, x.data as *mut ())
        } else {
            core::ptr::null()
        }
    }
}
