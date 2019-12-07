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
