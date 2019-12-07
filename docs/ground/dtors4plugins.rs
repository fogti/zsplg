use crate::stdlib::{fclose, free, FILE};

/* * dtors4plugins.h
 * (C) 2018 Erik Zscheile
 */
#[no_mangle]
pub unsafe extern "C" fn _Z10do_destroyP8_IO_FILE(f: *mut FILE) -> bool {
    return 0i32 == fclose(f);
}

/* fclose */
#[no_mangle]
pub unsafe extern "C" fn _Z10do_destroyPv(ptr: *mut libc::c_void) -> bool {
    free(ptr);
    return true;
}
