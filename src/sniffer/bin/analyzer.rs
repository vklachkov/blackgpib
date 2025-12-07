mod disk_request;
mod gpib_command;

use disk_request::Request as DiskRequest;
use gpib_command::GPIBCommand;
use std::{
    env::args,
    fs::File,
    io::{self, Read},
    process::ExitCode, time::Duration,
};

enum GPiBByte {
    Command { timestamp: Duration, cmd: GPIBCommand },
    Data { timestamp: Duration, byte: u8, eoi: bool },
}

struct DumpIterator {
    file: File,
    end_marker_found: bool,
}

impl DumpIterator {
    pub fn new(file: File) -> Self {
        Self {
            file,
            end_marker_found: false,
        }
    }
}

impl Iterator for DumpIterator {
    type Item = io::Result<GPiBByte>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end_marker_found {
            return None;
        }

        let mut buffer = [0; 5];

        if let Err(err) = self.file.read_exact(&mut buffer) {
            return Some(Err(err));
        }

        if buffer == [0; 5] {
            self.end_marker_found = true;
            return None;
        }

        let timestamp = (buffer[0] as u64) | ((buffer[1] as u64) << 8) | ((buffer[2] as u64) << 16);
        let timestamp = Duration::from_millis(timestamp);

        let byte = buffer[3];
        let atn = buffer[4] & (1 << 0b10) != 0;
        let eoi = buffer[4] & (1 << 0b01) != 0;

        let byte = if atn {
            let cmd = GPIBCommand::from(byte);
            GPiBByte::Command { timestamp, cmd }
        } else {
            GPiBByte::Data { timestamp, byte, eoi }
        };

        Some(Ok(byte))
    }
}

#[derive(Clone, Copy)]
enum State {
    Idle,
    ToDevice,
    FromDevice,
}

fn main() -> ExitCode {
    let Some(path) = args().nth(1) else {
        println!("Usage: gpib-dump-analyzer [FILE] [compact|full]");
        return ExitCode::FAILURE;
    };

    let compact = args().nth(2).as_deref() == Some("compact");

    let file = File::open(&path).expect("failed to open file");
    let mut dump_iter = DumpIterator::new(file);

    let mut state = State::Idle;
    let mut buffer = Vec::with_capacity(512);

    while let Some(byte) = dump_iter.next() {
        let byte = byte.expect("failed to read byte from dump");
        match byte {
            GPiBByte::Command { timestamp, cmd } => {
                if !compact {
                    println!("Laptop ({timestamp:?}) > {cmd:?}");
                }

                match cmd {
                    GPIBCommand::MLA(_) => state = State::ToDevice,
                    GPIBCommand::MTA(_) => state = State::FromDevice,
                    GPIBCommand::SPD => if compact {
                        buffer.clear();
                        state = State::Idle;
                    },
                    _ => state = State::Idle,
                };
            }
            GPiBByte::Data { timestamp, byte, eoi } => {
                buffer.push(byte);
                if eoi {
                    display_buffer(state, timestamp, &buffer);
                    buffer.clear();
                }
            }
        }
    }

    ExitCode::SUCCESS
}

fn display_buffer(state: State, timestamp: Duration, buffer: &[u8]) {
    match state {
        State::Idle => {
            println!("Idle ({timestamp:?}) > {buffer:02x?}");
        }
        State::ToDevice => {
            if let Ok(a) = DiskRequest::try_from(buffer) {
                println!();
                println!("Laptop ({timestamp:?}) > {a:?}");
            } else {
                println!("Laptop ({timestamp:?}) > {buffer:02x?}");
            }
        }
        State::FromDevice => {
            println!("Device ({timestamp:?}) > {buffer:02x?}");
        }
    }
}
