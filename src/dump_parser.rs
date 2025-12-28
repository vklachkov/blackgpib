#![allow(unused)]

mod disk_protocol;
mod gpib;
mod gpio;
mod logger;
mod time_utils;

use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
    time::Duration,
};

use disk_protocol::{DiskIdentity, Request as DiskRequest, StatusResponse as DiskStatusResponse};
use gpib::Command as GPIBCommand;

struct Args {
    compact: bool,
    path: PathBuf,
}

enum GPIBByte {
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
    type Item = io::Result<GPIBByte>;

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
        let atn = buffer[4] & 0b10 != 0;
        let eoi = buffer[4] & 0b01 != 0;

        let byte = if atn {
            let cmd = GPIBCommand::from(byte);
            GPIBByte::Command { timestamp, cmd }
        } else {
            GPIBByte::Data { timestamp, byte, eoi }
        };

        Some(Ok(byte))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    SerialPollProbing,
    SerialPollTalk(u8),
    DeviceListen(u8),
    DeviceTalk(u8),
}

fn main() {
    fix_broken_pipe();

    let Args { path, compact } = parse_args();

    let file = File::open(&path).expect("failed to open file");
    let mut dump_iter = DumpIterator::new(file);

    let mut state = State::Idle;
    let mut buffer = Vec::with_capacity(512);

    while let Some(byte) = dump_iter.next() {
        match byte.expect("failed to read byte from dump") {
            GPIBByte::Command { timestamp, cmd } => {
                display_command(timestamp, cmd, compact);

                state = match cmd {
                    GPIBCommand::MLA(dev) => State::DeviceListen(dev),
                    GPIBCommand::MTA(dev) => {
                        if state == State::SerialPollProbing {
                            State::SerialPollTalk(dev)
                        } else {
                            State::DeviceTalk(dev)
                        }
                    }
                    GPIBCommand::SPE => State::SerialPollProbing,
                    _ => State::Idle,
                };
            }
            GPIBByte::Data { timestamp, byte, eoi } => {
                buffer.push(byte);
                if eoi {
                    display_buffer(state, timestamp, &buffer, compact);
                    buffer.clear();
                } else if matches!(state, State::SerialPollTalk(_)) {
                    if !compact {
                        display_buffer(state, timestamp, &buffer, compact);
                    }
                    buffer.clear();
                }
            }
        }
    }
}

fn fix_broken_pipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

fn parse_args() -> Args {
    use bpaf::{Parser, construct, long, positional};

    let compact = long("compact").help("Hide all GPIB commands and raw data").switch();
    let path = positional("PATH");

    construct!(Args { compact, path })
        .to_options()
        .descr("GPIB Peripheral Emulator for GRiD Compass")
        .run()
}

fn display_command(timestamp: Duration, cmd: GPIBCommand, compact: bool) {
    if !compact {
        let timestamp = format!("{:02}.{:03}s", timestamp.as_secs(), timestamp.as_millis() % 1000);
        println!("💻 Compass ({timestamp}) > {cmd:?}");
    }
}

fn display_buffer(state: State, timestamp: Duration, buffer: &[u8], compact: bool) {
    let timestamp = format!("{}.{:03}s", timestamp.as_secs(), timestamp.as_millis() % 1000);

    match state {
        State::Idle => {
            println!("⚠️ Idle ({timestamp}) > {buffer:02x?}");
        }
        State::SerialPollProbing => {
            println!("⚠️ Broken serial poll ({timestamp}) > {buffer:02x?}")
        }
        State::SerialPollTalk(dev) => {
            println!("📟 Device #{dev} ({timestamp}) > {buffer:02x?}");
        }
        State::DeviceListen(dev) => {
            if let Ok(a) = DiskRequest::try_from(buffer) {
                println!("💻 Compass to #{dev} ({timestamp}) > {a:?}");
                if !compact {
                    println!("\tRaw: {buffer:02x?}");
                }
            } else {
                println!("💻 Compass to #{dev} ({timestamp}) > {buffer:02x?}");
            }
        }
        State::DeviceTalk(dev) => {
            if let Ok(status_response) = DiskStatusResponse::try_from_bytes(&buffer) {
                println!("📟 Device #{dev} ({timestamp}) > {status_response:?}");
            } else if let Ok(identity) = DiskIdentity::try_from_bytes(&buffer) {
                println!("📟 Device #{dev} ({timestamp}) > {identity:?}");
                if !compact {
                    println!("\tRaw: {buffer:02x?}");
                }
            } else {
                println!("📟 Device #{dev} ({timestamp}) > {buffer:02x?}");
            }

            if compact {
                println!();
            }
        }
    }
}
