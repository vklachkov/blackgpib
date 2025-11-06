mod device_info;
mod disk_identity;
mod disk_request;
mod gpib;
mod gpib_command;
mod gpio;
mod listener;
mod logger;
mod talker;
mod utils;

use std::time::Duration;

use crate::{
    device_info::*,
    disk_request::{Request, RequestCode},
    gpib_command::GPIBCommand,
    listener::{Listener, ListeningResult},
    talker::Talker,
    utils::busy_wait,
};

#[derive(Clone, Debug, Default)]
enum DeviceState {
    #[default]
    Idle,
    Read(DeviceReadState),
    Write(DeviceWriteState),
}

#[derive(Clone, Copy, Debug)]
enum DeviceReadState {
    SendData { sector_number: u32, data_size: u16 },
}

#[derive(Clone, Debug)]
enum DeviceWriteState {
    WaitData { sector_number: u32, data_size: u16 },
    DataReceived { sector_number: u32, bytes: Vec<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeviceResponse {
    Nothing,
    Identity { size: usize },
    SerialPoll { has_data: bool },
    WriteResponse {/* Always send 7 zeros for now */},
    ImagePart { sector_number: u32, data_size: u16 },
}

fn main() {
    logger::configure();

    log::info!("BlackGPiB started");

    log::debug!("Reset all pins to Z-State...");
    gpio::reset_all();

    let image = std::fs::read("GRIDOS.img").unwrap();
    let mut device_state = DeviceState::Idle;

    loop {
        log::debug!("Start listening bus");
        let result = listen_until_talk(&mut device_state);

        log::debug!("Start talking to bus");
        let mut talker = Talker::new();

        match result {
            DeviceResponse::Nothing => panic!("Nothing?????????????"),
            DeviceResponse::Identity { size } => {
                talker.send_bytes(&IDENTITY[..size], true);
            }
            DeviceResponse::SerialPoll { has_data } => {
                let srq_response = if has_data { 0x4f } else { 0x0f };
                talker.send_bytes(&[srq_response], false);
            }
            DeviceResponse::WriteResponse { .. } => {
                talker.send_bytes(&[0, 0, 0, 0, 0, 0, 0], true);
            }
            DeviceResponse::ImagePart {
                sector_number,
                data_size,
            } => {
                let sector_offset = sector_number as usize * 512;
                let part = &image[sector_offset..sector_offset + data_size as usize];
                talker.send_bytes(&part, true);
            }
        }
    }
}

fn listen_until_talk(state: &mut DeviceState) -> DeviceResponse {
    let mut listener = Listener::new(ADDRESS as u8);
    let mut result = DeviceResponse::Nothing;

    loop {
        match listener.listen() {
            ListeningResult::Continue => {}
            ListeningResult::Command(cmd) => {
                log::info!("Listener catch command {cmd:?}");
                match cmd {
                    GPIBCommand::SPE => {
                        if matches!(
                            state,
                            DeviceState::Read(DeviceReadState::SendData { .. })
                                | DeviceState::Write(DeviceWriteState::DataReceived { .. })
                        ) {
                            result = DeviceResponse::SerialPoll { has_data: true };
                        } else {
                            println!("NOT MY SRQ???");
                            result = DeviceResponse::SerialPoll { has_data: false };
                        }
                    }
                    GPIBCommand::MTA(address) => {
                        if address == ADDRESS as u8 {
                            if result != DeviceResponse::Nothing {
                                return result;
                            } else {
                                let state = std::mem::take(state);
                                match state {
                                    DeviceState::Idle => {
                                        panic!("Can't talk in idle state!");
                                    }
                                    DeviceState::Read(substate) => match substate {
                                        DeviceReadState::SendData {
                                            sector_number,
                                            data_size,
                                        } => {
                                            return DeviceResponse::ImagePart {
                                                sector_number,
                                                data_size,
                                            };
                                        }
                                    },
                                    DeviceState::Write(substate) => match substate {
                                        DeviceWriteState::WaitData {
                                            sector_number,
                                            data_size,
                                        } => {
                                            panic!("Can't talk without data");
                                        }
                                        DeviceWriteState::DataReceived { sector_number, bytes } => {
                                            println!("Yeee, received data!");
                                            return DeviceResponse::WriteResponse {};
                                        }
                                    },
                                }
                            }
                        } else {
                            listener.wait_next_command();
                        }
                    }
                    _ => {}
                }
            }
            ListeningResult::AnotherDeviceListen(_) => {
                listener.wait_next_command();
            }
            ListeningResult::Done(bytes) => match state {
                DeviceState::Idle => {
                    let request = parse_request(&bytes).expect("valid request");
                    match request.code {
                        RequestCode::GetStatus => {
                            result = DeviceResponse::Identity {
                                size: request.data_size as usize,
                            };
                        }
                        RequestCode::Read => {
                            listener.srq_feedback();
                            *state = DeviceState::Read(DeviceReadState::SendData {
                                sector_number: request.sector_number,
                                data_size: request.data_size,
                            });
                        }
                        RequestCode::Write => {
                            *state = DeviceState::Write(DeviceWriteState::WaitData {
                                sector_number: request.sector_number,
                                data_size: request.data_size,
                            });
                        }
                        _ => panic!("Unexpected request {request:?}"),
                    }
                }
                DeviceState::Read(substate) => {
                    panic!("Unexpected bytes in read bytes: {bytes:02x?}");
                }
                DeviceState::Write(substate) => match substate {
                    DeviceWriteState::WaitData {
                        sector_number,
                        data_size,
                    } => {
                        assert_eq!(bytes.len(), *data_size as usize, "Unexpected request size: {bytes:02?}");
                        listener.srq_feedback();
                        *state = DeviceState::Write(DeviceWriteState::DataReceived {
                            sector_number: *sector_number,
                            bytes,
                        });
                    }
                    _ => {
                        panic!("Unexpected bytes in write state: {bytes:02x?}");
                    }
                },
            },
        }

        busy_wait(Duration::from_micros(10));
    }
}

fn parse_request(raw: &[u8]) -> Option<Request> {
    match Request::try_from(raw) {
        Ok(value) => {
            log::debug!("Successfully parse {value:?}");
            Some(value)
        }
        Err(err) => {
            log::debug!("Failed to parse request {raw:02x?}: {err}");
            None
        }
    }
}
