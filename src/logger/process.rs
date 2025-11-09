use std::thread;

use super::{LogEntry, LogLevel, RingBuffer};

pub(crate) unsafe fn logger_process(ringbuf: *mut RingBuffer) {
    let parent_pid = unsafe { libc::getppid() };
    
    loop {
        // Проверяем, не умер ли родительский процесс
        let current_ppid = unsafe { libc::getppid() };
        if current_ppid == 1 || current_ppid != parent_pid {
            // Родитель умер (стал init или изменился)
            break;
        }
        
        // Читаем из ring buffer
        if let Some(entry) = unsafe { super::ringbuf::pop(ringbuf) } {
            format_and_print(&entry);
        }
        
        thread::yield_now();
    }
}

fn format_and_print(entry: &LogEntry) {
    let level_str = match entry.level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    };
    
    // Извлекаем file как строку
    let file_len = entry.file.iter().position(|&b| b == 0).unwrap_or(entry.file.len());
    let file = if file_len > 0 {
        String::from_utf8_lossy(&entry.file[..file_len])
    } else {
        std::borrow::Cow::Borrowed("")
    };
    
    // Извлекаем message как строку
    let msg_len = entry.message.iter().position(|&b| b == 0).unwrap_or(entry.message.len());
    let message = if msg_len > 0 {
        String::from_utf8_lossy(&entry.message[..msg_len])
    } else {
        std::borrow::Cow::Borrowed("")
    };
    
    // Форматируем временную метку (наносекунды -> секунды.миллисекунды.микросекунды.наносекунды)
    let total_ns = entry.timestamp_ns;
    let seconds = total_ns / 1_000_000_000;
    let remaining_ns = total_ns % 1_000_000_000;
    let milliseconds = remaining_ns / 1_000_000;
    let remaining_ns = remaining_ns % 1_000_000;
    let microseconds = remaining_ns / 1_000;
    let nanoseconds = remaining_ns % 1_000;
    
    println!(
        "[{}sec {:03}ms {:03}us {:03}ns] {} {}:{} > {}",
        seconds, milliseconds, microseconds, nanoseconds,
        level_str,
        file,
        entry.line,
        message
    );
}

