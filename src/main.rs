mod disk_identity;
mod disk_request;
mod gpib;
mod gpib_command;
mod gpio;
mod listener;
mod logger;
mod talker;
mod utils;

use crate::{
    disk_identity::DiskIdentity,
    disk_request::{Request, RequestCode},
    gpib::SupportedDeviceAddress,
    gpib_command::GPIBCommand,
    listener::{Listener, ListeningResult},
    talker::Talker,
};

const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::ExternalFloppy;

const IDENTITY: DiskIdentity = DiskIdentity {
    sector_size: 512,
    log_sector_size: 504,
    sector_count: 720,
    drive_ready: true,
    bit_map: 0b100100000,
    dir_fid: 289,
    min_dir_pages: 1,
    flush: 0,
    dev_name: *b"48 TPI DS DD FLOPPY    30237-00\0",
    // Extracted from real floppy. Weird values, but works.
    bytes_per_sector: 2306,
    sectors_per_track: 2304,
    tracks_per_cylinder: 512,
    // Unused by floppy.
    unknown: [0; 4],
};

const IDENTITY_BYTES: [u8; 56] = IDENTITY.into_bytes();

fn main() {
    logger::configure();

    log::info!("BlackGPiB started");

    log::debug!("Reset all pins to Z-State...");
    gpio::reset_all();

    let image = std::fs::read("GRIDOS.img").unwrap();

    let mut sector = 0;
    let mut size = 0;

    loop {
        log::debug!("Start listening bus");
        let (request, serial_poll) = listen();

        log::debug!("Start talking to bus");
        let mut talker = Talker::new();

        let Some(request) = request.and_then(parse_request) else {
            if serial_poll {
                log::debug!("Oooooooooo???");
                talker.send_bytes(&[0x4f], false);
                log::debug!("Ooooo");
            } else {
                let part = &image[(sector * 512)..(sector * 512 + size)];
                log::debug!("Sending part of image {sector} {size}: {part:02x?}");
                talker.send_bytes(part, true);
                log::debug!("Success");
            }
            continue;
        };

        log::debug!("Request from Compass: {request:?}");

        let response: &[u8] = match request.code {
            RequestCode::GetStatus => &IDENTITY_BYTES[..request.data_size as usize],
            RequestCode::Read => {
                // TODO: Check sector
                // Return sector
                sector = request.sector_number as usize;
                size = request.data_size as usize;
                log::debug!("Yeeeeeeeee, we must read {} bytes", request.data_size);
                &[0x4f]
            }
            RequestCode::Write => {
                // Patch image
                unimplemented!("Emulator is read only now")
            }
            _ => {
                unimplemented!("Unsupported command");
            }
        };

        log::debug!("Sending response {response:02x?}");
        talker.send_bytes(response, true);
        log::debug!("Response sent successfull");
    }
}

fn listen() -> (Option<Vec<u8>>, bool) {
    let mut listener = Listener::new(ADDRESS as u8);
    let mut serial_poll = false;
    let mut buffer = Vec::new();

    loop {
        let result = listener.listen();
        match result {
            ListeningResult::Continue => {
                continue;
            }
            ListeningResult::Command(cmd) => {
                log::info!("Listener catch command {cmd:?}");
                match cmd {
                    GPIBCommand::DCL | GPIBCommand::SDC => {
                        listener.reset();
                    }
                    GPIBCommand::SPE => {
                        serial_poll = true;
                    }
                    GPIBCommand::SPD => {
                        serial_poll = false;
                    }
                    GPIBCommand::MTA(address) => {
                        if address == ADDRESS as u8 {
                            let buffer = if buffer.is_empty() { None } else { Some(buffer) };
                            return (buffer, serial_poll);
                        } else {
                            // unused for now
                            // listener.wait_next_command();
                        }
                    }
                    _ => {}
                }
            }
            ListeningResult::AnotherDeviceListen(_) => {
                // unused for now
                // listener.wait_next_command();
            }
            ListeningResult::Done(bytes) => {
                log::debug!("Save {} bytes for future parsing", bytes.len());
                // Hack!
                if bytes.get(0) == Some(&4) {
                    listener.srq_low();
                }
                buffer = bytes;
            }
        }
    }
}

fn parse_request(raw: Vec<u8>) -> Option<Request> {
    match Request::try_from(raw.as_slice()) {
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
