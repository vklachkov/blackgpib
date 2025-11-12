mod buffer;
mod macros;
mod process;
mod structs;

use std::{
    ffi::CStr,
    fmt::Arguments,
    io::{self, Write},
    sync::{LazyLock, Mutex, OnceLock},
    time::Instant,
};

use ringbuf::traits::Producer;

use self::{
    buffer::{BufferProducer, SharedRingBuffer},
    process::fork_logger,
    structs::*,
};

pub use self::structs::LogLevel;

pub struct Logger {
    start_time: Instant,
    ringbuf: SharedRingBuffer<LogEntry>,
}

const SHMEM_NAME: &CStr = c"/blackgpib_logbuf";

static LOGGER: LazyLock<Logger> = LazyLock::new(|| {
    let start_time = Instant::now();

    // SAFETY: Initialization is performed only once and only in the main process.
    let ringbuf = unsafe { SharedRingBuffer::new(SHMEM_NAME) };

    Logger { start_time, ringbuf }
});

struct LogProducer {
    level: LogLevel,
    buffer: BufferProducer<'static, LogEntry>,
}

static LOG_PRODUCER: OnceLock<Mutex<LogProducer>> = OnceLock::new();

/// Initializes the logger and forks a separate process to display logs.
pub fn setup(level: LogLevel) {
    assert!(LOG_PRODUCER.get().is_none(), "logger is already initialized");

    let (prod, cons) = unsafe { LOGGER.ringbuf.split_ref() };

    fork_logger(cons);

    _ = LOG_PRODUCER.set(Mutex::new(LogProducer { level, buffer: prod }));
}

/// Adds a message to the queue. If the queue is full, the message will be lost.
pub fn log(instant: Instant, file: &'static str, line: u32, level: LogLevel, message: Arguments<'_>) {
    let mut log_producer = LOG_PRODUCER
        .get()
        .expect("logger was not initialized")
        .lock()
        .expect("failed to acquire lock on log queue");

    if (level as u8) < (log_producer.level as u8) {
        return;
    }

    let mut entry = LogEntry {
        timespan: instant - LOGGER.start_time,
        file: [0u8; _],
        line,
        level,
        message: [0u8; _],
    };

    let mut file_cursor = io::Cursor::new(entry.file.as_mut_slice());
    _ = file_cursor.write(file.as_bytes());
    entry.file[entry.file.len() - 1] = 0;

    let mut msg_cursor = io::Cursor::new(entry.message.as_mut_slice());
    _ = msg_cursor.write_fmt(message);
    entry.message[entry.message.len() - 1] = 0;

    _ = log_producer.buffer.try_push(entry);
}
