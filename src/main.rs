mod disk_request;
mod gpib;
mod gpio;
mod listener;
mod logger;
mod message;
mod talker;

use crate::{
    disk_request::{Request, RequestCode},
    gpib::SupportedDeviceAddress,
    listener::{Listener, ListeningResult},
    talker::Talker,
};

const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::ExternalFloppy;

// TODO: Represent as struct not bytes.
// Description here http://deltacxx.insomnia247.nl/projects/gridcompass/disk_info.txt.
const IDENTITY: [u8; 52] = [
    0x00, 0x02, 0xf8, 0x01, 0xd0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38,
    0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4c, 0x4f, 0x50, 0x50,
    0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x30, 0x32, 0x33, 0x37, 0x2d, 0x30, 0x30, 0x00, 0x02,
    0x09, 0x00, 0x02, 0x00,
];

fn main() {
    logger::configure();

    log::info!("BlackGPiB started");

    log::debug!("Reset all pins...");
    gpio::reset_all();

    let image = std::fs::read("GRIDOS.img").unwrap();

    let mut sector = 0;
    let mut size = 0;

    loop {
        log::debug!("Start listening bus");
        let result = listen();

        log::debug!("Start talking to bus");
        let mut talker = Talker::new();

        let Some((request, serial_poll)) = result.and_then(|(r, s)| Some((parse_request(r)?, s)))
        else {
            log::debug!("Sending part of image");
            talker.send_bytes(&image[(sector * 512)..(sector * 512 + size)]);
            log::debug!("Success");
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

fn listen() -> Option<(Vec<u8>, bool)> {
    let mut listener = Listener::new(ADDRESS as u8);
    let mut request = None;
    let mut serial_poll = false;

    loop {
        match listener.listen() {
            ListeningResult::Done { bytes } => {
                log::debug!("Bytes {bytes:02x?}! But we must continue to listen");
                // Hack!
                if bytes.get(0) == Some(&4) {
                    listener.srq_low();
                }
                request = Some(bytes);
            }
            ListeningResult::Unhandled { byte, is_command } if is_command => {
                if message::is_mta(byte, ADDRESS as u8) {
                    log::debug!("MTA received, drop listener");
                    listener.srq_high();
                    let a = request.map(|r| (r, serial_poll));
                    return a;
                } else if message::is_spe(byte) {
                    log::debug!("Serial Poll Enable");
                    serial_poll = true;
                } else if message::is_spd(byte) {
                    log::debug!("Serial Poll Disable");
                    serial_poll = false;
                } else if message::is_dcl(byte) {
                    log::debug!("Device CLear received");
                    // listener.reset();
                } else if message::is_unt(byte) {
                    log::debug!("Untalk received")
                } else {
                    log::debug!("Unrecognized command {byte:#010b}");
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
