mod disk_request;
mod gpib;
mod gpio;
mod listener;
mod message;
mod talker;

use crate::{
    disk_request::{Request, RequestCode},
    listener::{Listener, ListeningResult},
    talker::Talker,
};

// 4 for HDD 10MB, 5/6 for Floppy 5.25.
const ADDRESS: u8 = 5;

// TODO: Represent as struct not bytes.
// Description here http://deltacxx.insomnia247.nl/projects/gridcompass/disk_info.txt.
const IDENTITY: [u8; 56] = [
    0x00, 0x02, 0xf8, 0x01, 0xD0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38,
    0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4c, 0x4f, 0x50, 0x50,
    0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x32, 0x33, 0x37, 0x2d, 0x30, 0x30, 0x00, 0x02, 0x09,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

enum DeviceState {
    Idle,
}

fn main() {
    gpio::reset_all();

    loop {
        let request = listen();

        let talker = Talker::new(ADDRESS);

        let Some(request) = request else {
            unimplemented!("talk without request???");
        };

        let response = match request.code {
            RequestCode::GetStatus => {
                // Return identity
            }
            RequestCode::Read => {
                // TODO: Check sector
                // Return sector
            }
            RequestCode::Write => {
                // Patch image
            }
            _ => {
                // Log as unsupported
            }
        };
    }
}

fn listen() -> Option<Request> {
    let mut listener = Listener::new(ADDRESS);
    let mut request = None;

    loop {
        match listener.listen() {
            ListeningResult::Done { bytes } => {
                request = Some(bytes);
            }
            ListeningResult::Unhandled { byte, is_command } if is_command => {
                if message::is_mta(byte, ADDRESS) {
                    return request.and_then(parse_request);
                } else if message::is_dcl(byte) {
                    println!("DCL received, reset listener to Idle state");
                    listener.reset();
                } else {
                    println!("Unrecognized command {byte:#010b}");
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
            println!("Successfully parse {value:?}");
            Some(value)
        }
        Err(err) => {
            println!("Failed to parse request {raw:02x?}: {err}");
            None
        }
    }
}
