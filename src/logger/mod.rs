use std::io::Write;
use std::sync::{LazyLock, atomic::AtomicU32};
use std::time::Instant;

mod macros;
mod process;
mod ringbuf;

// Макросы экспортируются через #[macro_export] в macros.rs

const SHM_NAME: &str = "/blackgpib_logger";
const RING_BUFFER_SIZE: usize = 1024;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LogEntry {
    file: [u8; 64],
    line: u32,
    level: LogLevel,
    timestamp_ns: u128,
    message: [u8; 256],
}

#[repr(C)]
struct RingBuffer {
    read_pos: AtomicU32,
    write_pos: AtomicU32,
    entries: [LogEntry; RING_BUFFER_SIZE],
}

pub struct Logger {
    start_time: Instant,
    ringbuf: *mut RingBuffer,
}

// Безопасно для использования в lock-free ring buffer между процессами
unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

static LOGGER: LazyLock<Logger> = LazyLock::new(|| Logger {
    start_time: Instant::now(),
    ringbuf: unsafe { ringbuf::init_shared_memory() },
});

pub fn init() {
    // Инициализируем LazyLock
    let logger = &*LOGGER;
    let ringbuf = logger.ringbuf;

    // Форкаем процесс логгера
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Дочерний процесс - процесс логгера
        unsafe {
            process::logger_process(ringbuf);
            libc::_exit(0);
        }
    } else if pid < 0 {
        panic!("Failed to fork logger process");
    }
    // Родительский процесс продолжает работу
}

pub(crate) fn get_logger() -> &'static Logger {
    &*LOGGER
}

impl Logger {
    pub fn log(&self, instant: Instant, file: &str, line: u32, level: LogLevel, args: std::fmt::Arguments) {
        let timestamp_ns = instant.duration_since(self.start_time).as_nanos() as u128;

        let mut entry = LogEntry {
            file: [0; 64],
            line,
            level,
            timestamp_ns,
            message: [0; 256],
        };

        // Копируем file (максимум 63 символа + null terminator)
        let file_bytes = file.as_bytes();
        let file_len = file_bytes.len().min(63);
        entry.file[..file_len].copy_from_slice(&file_bytes[..file_len]);
        entry.file[file_len] = 0;

        // Форматируем message напрямую в буфер без аллокаций используя Cursor
        let mut cursor = std::io::Cursor::new(entry.message.as_mut_slice());
        let _ = cursor.write_fmt(args);
        let msg_len = cursor.position() as usize;
        let msg_len = msg_len.min(255);
        entry.message[msg_len] = 0;

        unsafe {
            ringbuf::push(self.ringbuf, &entry);
        }
    }
}
