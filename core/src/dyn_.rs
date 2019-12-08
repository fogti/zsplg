use super::{Wrapped, WrapperInner};
use dyn_sized::DynSized;

mod private {
    use core::ffi::c_void;
    use crate::WrapMeta;

    pub trait MetaWrapped: Sized {
        fn wrap(self) -> WrapMeta;
        fn unwrap(x: WrapMeta) -> Option<Self>;
        fn null() -> Self;
    }

    impl MetaWrapped for usize {
        fn wrap(self) -> WrapMeta {
            WrapMeta::Length(self)
        }

        fn unwrap(x: WrapMeta) -> Option<Self> {
            if let WrapMeta::Length(y) = x {
                Some(y)
            } else {
                None
            }
        }

        fn null() -> Self {
            0
        }
    }

    impl MetaWrapped for *mut () {
        fn wrap(self) -> WrapMeta {
            WrapMeta::TraitObject(self as *mut c_void)
        }

        fn unwrap(x: WrapMeta) -> Option<Self> {
            if let WrapMeta::TraitObject(y) = x {
                Some(y as *mut ())
            } else {
                None
            }
        }

        fn null() -> Self {
            core::ptr::null_mut()
        }
    }
}

unsafe impl<T> Wrapped for T
where
    T: ?Sized + dyn_sized::DynSized,
    T::Meta: Copy + private::MetaWrapped,
{
    fn wrap(x: *const Self) -> WrapperInner {
        let (meta, ptr) = DynSized::disassemble(x);
        WrapperInner {
            data: ptr as *const super::c_void,
            meta: private::MetaWrapped::wrap(meta),
        }
    }

    fn as_ptr(x: &WrapperInner) -> *const Self {
    let (meta, data) = if let Some(meta) = <T::Meta as private::MetaWrapped>::unwrap(x.meta) {
        (meta, x.data as *mut ())
    } else {
        (
            <T::Meta as private::MetaWrapped>::null(),
            core::ptr::null_mut(),
        )
    };
        DynSized::assemble(meta, data)
    }
}
