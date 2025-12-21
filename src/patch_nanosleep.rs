use std::time::Duration;

use libc::{c_int, timespec};

unsafe extern "C" {
    unsafe fn __real_nanosleep(req: *const timespec, rem: *mut timespec) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __wrap_nanosleep(req: *const timespec, _rem: *mut timespec) -> c_int {
    // TODO: add safety comment
    let req = unsafe { &*req };

    // TODO: add cast check
    let duration = Duration::new(req.tv_sec as _, req.tv_nsec as _);

    crate::utils::busy_wait(duration);

    0
}
