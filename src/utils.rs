use std::time::{Duration, Instant};

/// Waits for the specified time without context switching.
pub fn busy_wait(duration: Duration) {
    let start = Instant::now();
    while Instant::now() - start < duration {}
}
