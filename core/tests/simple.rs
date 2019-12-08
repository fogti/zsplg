extern crate alloc;

#[test]
fn test_null() {
    let mut wrap = zsplg_core::Wrapper::null();
    wrap.call_dtor();
}

#[test]
fn test_one() {
    let mut wrap = unsafe { zsplg_core::Wrapper::new(1usize) };
    assert_eq!(*wrap.try_cast_sized::<usize>().unwrap(), 1usize);
    wrap.call_dtor();
}

#[test]
fn test_onedw() {
    let wrap = zsplg_core::WrapWithDrop(unsafe { zsplg_core::Wrapper::new(1usize) });
    assert_eq!(*wrap.try_cast_sized::<usize>().unwrap(), 1usize);
}

#[test]
fn test_arc() {
    use alloc::sync::Arc;
    let wrap = zsplg_core::WrapWithDrop(unsafe { zsplg_core::Wrapper::new(1usize) });
    let x2 = wrap.try_cast_raw::<zsplg_core::WrapSized<usize>>().unwrap();
    assert_eq!((*x2).0, 1usize);
    assert_eq!(*wrap.try_cast_sized::<usize>().unwrap(), 1usize);
    assert_eq!(Arc::strong_count(&x2), 2);
    core::mem::drop(wrap);

    assert_eq!(Arc::strong_count(&x2), 1);
    assert_eq!((*x2).0, 1usize);
}
