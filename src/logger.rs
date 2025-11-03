use std::{sync::LazyLock, time::Instant};

use log::*;

static LOGGER: LazyLock<Logger> = LazyLock::new(|| Logger {
    start_time: Instant::now(),
});

pub fn configure() {
    set_max_level(LevelFilter::Debug);
    set_logger(&*LOGGER).unwrap();
}

struct Logger {
    start_time: Instant,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let now = Instant::now() - self.start_time;
        let ms = now.as_millis();
        let us = now.as_micros() % 1000;

        let file = record.file().unwrap_or("???");
        let line = record.line().unwrap_or(0);

        println!("[{ms}ms {us:03}us] {file}:{line} > {}", record.args());
    }

    fn flush(&self) {}
}
