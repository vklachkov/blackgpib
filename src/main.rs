mod disk_request;
mod gpib;
mod gpio;
mod listener;
mod logger;
mod gpib_command;
mod talker;

use crate::{
    disk_request::{Request, RequestCode},
    gpib::SupportedDeviceAddress,
    listener::{Listener, ListeningResult},
    gpib_command::GPIBCommand,
    talker::Talker,
};

const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::ExternalFloppy;

// TODO: Represent as struct not bytes.
// Description here http://deltacxx.insomnia247.nl/projects/gridcompass/disk_info.txt.
const IDENTITY: [u8; 52] = [
    // 0x00, 0x02, 0xf8, 0x01, 0xd0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38,
    // 0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4c, 0x4f, 0x50, 0x50,
    // 0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x30, 0x32, 0x33, 0x37, 0x2d, 0x30, 0x30, 0x00, 0x02,
    0x00, 0x02, 0xf8, 0x01, 0xD0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00,
    0x34, 0x38, 0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46,
    0x4c, 0x4f, 0x50, 0x50, 0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x32, 0x33, 0x37,
    0x2d, 0x30, 0x30, 0x00, 0x02, 0x09, 0x00, 0x09, 0x00, 0x02
];

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
                talker.send_bytes(&[0x4f]);
                log::debug!("Ooooo");
            } else {
                let part = &image[(sector * 512)..(sector * 512 + size)];
                log::debug!("Sending part of image {sector} {size}: {part:02x?}");
                talker.send_bytes(part);
                log::debug!("Success");
            }
            continue;
        };

        log::debug!("Request from Compass: {request:?}");

        let response: &[u8] = match request.code {
            RequestCode::GetStatus => &IDENTITY,
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
        talker.send_bytes(response);
        log::debug!("Response sent successfull");
    }
}

fn listen() -> (Option<Vec<u8>>, bool) {
    let mut listener = Listener::new(ADDRESS as u8);
    let mut request = None;
    let mut serial_poll = false;

    loop {
        match listener.listen() {
            ListeningResult::Done { bytes } => {
                log::debug!("Save {} bytes for future parsing", bytes.len());
                // Hack!
                if bytes.get(0) == Some(&4) {
                    listener.srq_low();
                }
                request = Some(bytes);
            }
            ListeningResult::UnhandledCommand { cmd } => {
                log::info!("Listener catch command {cmd:?}");

                if cmd == GPIBCommand::MTA(ADDRESS as u8) {
                    return (request, serial_poll);
                } else if cmd == GPIBCommand::SPE {
                    serial_poll = true;
                } else if cmd == GPIBCommand::SPD {
                    serial_poll = false;
                }
            }
            _ => {
                continue;
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
