#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "dyn_sized")]
mod dyn_;

use alloc::sync::Arc;
use core::ffi::c_void;

#[allow(non_camel_case_types)]
pub type c_bool = u8;
#[allow(non_upper_case_globals)]
pub const c_false: c_bool = 0;
#[allow(non_upper_case_globals)]
pub const c_true: c_bool = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WrapMeta {
    None,
    Bytes(usize),
    Length(usize),
    TraitObject(*mut c_void),
}

#[repr(C)]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapperInner {
    pub data: *const c_void,
    pub meta: WrapMeta,
}

/// The main ffi wrapper type;
/// the .destroy function is expected to be called manually
///
/// NOTE about data layout and semantics:
/// This type should change almost never, if it changes,
/// a major bump of the this crate version is necessary.
/// Only versions of this library with the same major
/// version should be used inside the same application.
///
/// Once an object of this type is constructed,
/// it is not legal to change the value of any content of this struct,
/// but this struct may be resetted (only while destructing).
#[repr(C)]
#[must_use]
#[derive(Clone, Copy)]
pub struct Wrapper {
    pub inner: WrapperInner,

    pub typ: Option<core::any::TypeId>,

    /// The .destroy function may only be called with .inner
    /// as argument and must be called at most once
    pub destroy: Option<extern "C" fn(*mut WrapperInner) -> bool>,

    pub klone: Option<extern "C" fn(&WrapperInner) -> WrapperInner>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct WrapSized<T: 'static>(pub T);

pub unsafe trait Wrapped: 'static {
    fn wrap(x: *const Self) -> WrapperInner;
    fn as_ptr(x: &WrapperInner) -> *const Self;
}

unsafe impl<T> Wrapped for WrapSized<T>
where
    T: Sized + 'static,
{
    fn wrap(x: *const Self) -> WrapperInner {
        WrapperInner {
            data: x as *const c_void,
            meta: WrapMeta::Bytes(core::mem::size_of::<T>()),
        }
    }

    fn as_ptr(x: &WrapperInner) -> *const Self {
        x.data as *const Self
    }
}

/// This function is the default destructor
extern "C" fn ffiwrap_destroy<T>(data: *mut WrapperInner) -> bool
where
    T: ?Sized + Wrapped,
{
    if let Some(inner) = unsafe { data.as_mut() } {
        let ret = if inner.data.is_null() {
            false
        } else {
            let real_dtor = || {
                core::mem::drop(unsafe { Arc::from_raw(<T as Wrapped>::as_ptr(&inner)) });
            };

            #[cfg(not(feature = "std"))]
            real_dtor();
            #[cfg(feature = "std")]
            let ret = std::panic::catch_unwind(real_dtor).is_ok();
            #[cfg(not(feature = "std"))]
            let ret = true;
            ret
        };

        // To catch 'use-after-free' bugs regardless of the return
        // value of 'destroy', we always reset the 'data' ptr.
        inner.data = core::ptr::null();

        // Reset the rest of this struct
        inner.meta = WrapMeta::None;

        ret
    } else {
        false
    }
}

/// This function is the default klone method
extern "C" fn ffiwrap_klone<T>(inner: &WrapperInner) -> WrapperInner
where
    T: ?Sized + Wrapped,
{
    let tmp = unsafe { Arc::from_raw(<T as Wrapped>::as_ptr(inner)) };
    // increment the reference count by one because the original
    // reference is preserved, but `Arc::from_raw` expects that
    // we moved the reference
    let ret = tmp.clone();
    core::mem::forget(tmp);
    <T as Wrapped>::wrap(Arc::into_raw(ret))
}

impl WrapperInner {
    /// Constructs an empty inner ffi wrapper
    pub fn null() -> Self {
        Self {
            data: core::ptr::null_mut(),
            meta: WrapMeta::None,
        }
    }
}

impl Wrapper {
    /// This is a convenient wrapper, which moves T to the heap
    /// and then calls [`Wrapper::from`].
    pub unsafe fn new<T: 'static>(x: T) -> Self {
        Self::from(Arc::new(WrapSized(x)))
    }

    /// Constructs an empty ffi wrapper
    pub fn null() -> Self {
        Self {
            inner: WrapperInner::null(),
            typ: None,
            destroy: None,
            klone: None,
        }
    }

    /// Checks if an ffi wrapper is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.destroy.is_none() && self.inner == WrapperInner::null()
    }

    /// This function constructs a new Wrapper.
    ///
    /// # Safety
    /// This function allows possible violations of constraints on T
    /// if used incorrectly (e.g. double free).
    pub unsafe fn from<T>(x: Arc<T>) -> Self
    where
        T: ?Sized + Wrapped,
    {
        Self {
            inner: Wrapped::wrap(Arc::into_raw(x)),
            typ: Some(core::any::TypeId::of::<T>()),
            destroy: Some(ffiwrap_destroy::<T>),
            klone: Some(ffiwrap_klone::<T>),
        }
    }

    pub fn try_ptr_clone(&self) -> Option<Wrapper> {
        self.klone.map(|i| Wrapper {
            inner: i(&self.inner),
            typ: self.typ.clone(),
            destroy: self.destroy.clone(),
            klone: Some(i),
        })
    }

    /// This function only works for native rust types
    /// and only when the Wrapper was constructed using
    /// [`Wrapper::from`].
    #[inline(always)]
    fn is_of_type<T>(&self) -> bool
    where
        T: ?Sized + 'static,
    {
        self.typ == Some(core::any::TypeId::of::<T>())
    }

    /// convenient wrapper around [`Wrapper::is_of_type`]
    fn map_on_type<T, R, F>(&self, f: F) -> Option<R>
    where
        T: ?Sized + Wrapped,
        F: FnOnce(*const T) -> Option<R>,
    {
        if self.is_of_type::<T>() {
            f(<T as Wrapped>::as_ptr(&self.inner))
        } else {
            None
        }
    }

    /// This function extracts the inner `Arc` without
    /// incrementing the reference count
    ///
    /// This function only works for native rust types
    /// and only when the Wrapper was constructed using
    /// [`Wrapper::from`].
    pub fn try_unwrap<T>(self) -> Result<Arc<T>, Self>
    where
        T: ?Sized + Wrapped,
    {
        if self.is_of_type::<T>() {
            Ok(unsafe { Arc::from_raw(<T as Wrapped>::as_ptr(&self.inner)) })
        } else {
            Err(self)
        }
    }

    /// This function allows casting to the inner type
    /// while ensuring a minimal level of type safety.
    /// It increments the reference counter of the inner object.
    /// Dropping the returned object decreases the reference counter.
    ///
    /// This function only works for native rust types
    /// and only when the Wrapper was constructed using
    /// [`Wrapper::from`].
    pub fn try_cast_raw<T>(&self) -> Option<Arc<T>>
    where
        T: ?Sized + Wrapped,
    {
        self.map_on_type::<T, _, _>(|ptr| {
            let tmp = unsafe { Arc::from_raw(ptr) };
            // increment the reference count by one because the original
            // reference is preserved, but `Arc::from_raw` expects that
            // we moved the reference
            let ret = tmp.clone();
            core::mem::forget(tmp);
            Some(ret)
        })
    }

    /// This function allows casting to the original type
    /// while ensuring a minimal level of type safety.
    ///
    /// This function only works for native rust types
    /// and only when the Wrapper was constructed using
    /// [`Wrapper::from`].
    pub fn try_cast<T>(&self) -> Option<&T>
    where
        T: ?Sized + Wrapped,
    {
        self.map_on_type::<T, _, _>(|ptr| unsafe { ptr.as_ref() })
    }

    /// This function allows casting to the original type
    /// while ensuring a minimal level of type safety.
    ///
    /// This function only works for native rust types
    /// and only when the Wrapper was constructed using
    /// [`Wrapper::new`].
    pub fn try_cast_sized<T: 'static>(&self) -> Option<&T> {
        self.try_cast::<WrapSized<T>>().map(|x| &x.0)
    }

    pub fn call_dtor(&mut self) -> bool {
        let ret = self.destroy.take().map(|destroy| destroy(&mut self.inner));

        // To catch 'use-after-free' bugs regardless of the return
        // value of 'destroy', we always reset the 'data' ptr.
        self.inner.data = core::ptr::null_mut();

        // Reset the rest of this struct
        self.inner.meta = WrapMeta::None;
        self.klone = None;

        ret.unwrap_or(false)
    }
}

#[repr(transparent)]
pub struct WrapWithDrop(pub Wrapper);

impl Drop for WrapWithDrop {
    fn drop(&mut self) {
        self.0.call_dtor();
    }
}

impl core::ops::Deref for WrapWithDrop {
    type Target = Wrapper;

    fn deref(&self) -> &Wrapper {
        &self.0
    }
}
