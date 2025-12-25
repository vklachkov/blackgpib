use core::{arch::asm, time::Duration};

/// Waits for the specified time without context switching.
pub fn busy_wait(duration: std::time::Duration) {
    let ns: u64 = duration.as_nanos().try_into().expect("too big busy wait");

    unsafe {
        let frq: u64;
        let start: u64;
        let target: u64;

        asm!(
            "isb",
            "mrs {out_frq}, cntfrq_el0",
            "mrs {out_start}, cntvct_el0",
            out_frq = out(reg) frq,
            out_start = out(reg) start,
            options(nomem, nostack)
        );

        target = start + (frq * ns / 1_000_000_000);

        loop {
            let now: u64;
            asm!(
                "mrs {out_now}, cntvct_el0",
                out_now = out(reg) now,
                options(nomem, nostack)
            );

            if now >= target {
                break;
            }
        }
    }
}

pub fn measure<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let frq: u64;
    let start: u64;
    let stop: u64;

    unsafe {
        asm!(
            "isb",
            "mrs {out_frq}, cntfrq_el0",
            "mrs {out_start}, cntvct_el0",
            out_frq = out(reg) frq,
            out_start = out(reg) start,
            options(nomem, nostack)
        );
    }

    let result = f();

    unsafe {
        asm!(
            "isb",
            "mrs {out_stop}, cntvct_el0",
            out_stop = out(reg) stop,
            options(nomem, nostack)
        );
    }

    let ticks = stop.wrapping_sub(start);
    let nanos_u128 = (ticks as u128).saturating_mul(1_000_000_000u128) / (frq as u128);
    let duration = Duration::from_nanos(nanos_u128.min(u64::MAX as u128) as u64);

    return (result, duration);
}
