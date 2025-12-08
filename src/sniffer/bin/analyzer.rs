mod disk_identity;
mod disk_request;
mod disk_response;
mod gpib_command;

use std::{
    env::args,
    fs::File,
    io::{self, Read},
    process::ExitCode,
    time::Duration,
};

use disk_identity::DiskIdentity;
use disk_request::Request as DiskRequest;
use gpib_command::GPIBCommand;

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
        let atn = buffer[4] & 0b10 != 0;
        let eoi = buffer[4] & 0b01 != 0;

        let byte = if atn {
            let cmd = GPIBCommand::from(byte);
            GPiBByte::Command { timestamp, cmd }
        } else {
            GPiBByte::Data { timestamp, byte, eoi }
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

fn main() -> ExitCode {
    let Some(path) = args().nth(1) else {
        println!("Usage: gpib-dump-analyzer [FILE] [only_requests|full]");
        return ExitCode::FAILURE;
    };

    let only_requests = args().nth(2).as_deref() == Some("only_requests");

    let file = File::open(&path).expect("failed to open file");
    let mut dump_iter = DumpIterator::new(file);

    let mut state = State::Idle;
    let mut buffer = Vec::with_capacity(512);

    while let Some(byte) = dump_iter.next() {
        match byte.expect("failed to read byte from dump") {
            GPiBByte::Command { timestamp, cmd } => {
                display_command(timestamp, cmd, only_requests);

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
            GPiBByte::Data { timestamp, byte, eoi } => {
                buffer.push(byte);
                if eoi {
                    display_buffer(state, timestamp, &buffer, only_requests);
                    buffer.clear();
                } else if matches!(state, State::SerialPollTalk(_)) {
                    if !only_requests {
                        display_buffer(state, timestamp, &buffer, only_requests);
                    }
                    buffer.clear();
                }
            }
        }
    }

    ExitCode::SUCCESS
}

fn display_command(timestamp: Duration, cmd: GPIBCommand, only_requests: bool) {
    if !only_requests {
        let timestamp = format!("{:02}.{:03}s", timestamp.as_secs(), timestamp.as_millis() % 1000);
        println!("💻 Compass ({timestamp}) > {cmd:?}");
    }
}

fn display_buffer(state: State, timestamp: Duration, buffer: &[u8], only_requests: bool) {
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
                if !only_requests {
                    println!("\tRaw: {buffer:02x?}");
                }
            } else {
                println!("💻 Compass to #{dev} ({timestamp}) > {buffer:02x?}");
            }
        }
        State::DeviceTalk(dev) => {
            if buffer.len() == 7 {
                println!("📟 Device #{dev} ({timestamp}) > {buffer:02x?}");
            } else if let Ok(identity) = DiskIdentity::try_from_bytes(&buffer) {
                println!("📟 Device #{dev} ({timestamp}) > {identity:?}");
                if !only_requests {
                    println!("\tRaw: {buffer:02x?}");
                }
            } else {
                println!("📟 Device #{dev} ({timestamp}) > {buffer:02x?}");
            }

            if only_requests {
                println!();
            }
        }
    }
}
