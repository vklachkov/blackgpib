use std::{ffi::CStr, process::exit, thread::yield_now, time::Duration};

use ringbuf::traits::Consumer;

use crate::logger::{LogEntry, buffer::BufferConsumer};

pub fn fork_logger(cons: BufferConsumer<LogEntry>) {
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        panic!("failed to fork logger from blackgpib main process");
    }

    if pid > 0 {
        return;
    }

    process(cons);
    exit(0);
}

fn process(mut cons: BufferConsumer<LogEntry>) {
    loop {
        if let Some(entry) = cons.try_pop() {
            print_entry(entry);
        } else if is_parent_died() {
            break;
        } else {
            yield_now();
        }
    }
}

fn print_entry(entry: LogEntry) {
    let timestamp = format_duration(entry.timespan);
    let file = extract_filename(&entry);
    let line = entry.line;
    let level = entry.level.as_str();
    let msg = get_message(&entry);

    println!("{timestamp} {level} [{file}:{line}] {msg}")
}

fn format_duration(ts: Duration) -> String {
    let secs = ts.as_secs();
    let subsec_nanos = ts.subsec_nanos();
    let millis = subsec_nanos / 1_000_000;
    let micros = (subsec_nanos % 1_000_000) / 1_000;
    let nanos = subsec_nanos % 1_000;
    format!("{secs}sec {millis}ms {micros}.{nanos:03}us")
}

fn extract_filename(entry: &LogEntry) -> &str {
    let Ok(cfilename) = CStr::from_bytes_until_nul(&entry.file) else {
        return "???";
    };

    let Ok(filename) = cfilename.to_str() else {
        return "???";
    };

    filename.strip_prefix("src/").unwrap_or(filename)
}

fn get_message(entry: &LogEntry) -> &str {
    let Ok(cmessage) = CStr::from_bytes_until_nul(&entry.message) else {
        return "broken";
    };

    cmessage.to_str().unwrap_or("broken")
}

fn is_parent_died() -> bool {
    let ppid = unsafe { libc::getppid() };
    ppid == 1
}
