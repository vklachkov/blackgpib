use std::{
    thread,
    time::{Duration, Instant},
};

/// Increases process priority and pins it to the last CPU core.
pub fn configure_scheduler() {
    let available_cores = thread::available_parallelism().unwrap().get();
    let core = available_cores - 1;

    unsafe {
        let mut set = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(core, &mut set);
        libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &set);
    }

    unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -19) };
}

/// Waits for the specified time without context switching.
pub fn busy_wait(duration: Duration) {
    let start = Instant::now();
    while Instant::now() - start < duration {}
}
