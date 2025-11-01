mod disk_request;
mod gpib;
mod gpio;
mod listener;
mod message;

use crate::{
    disk_request::Request,
    listener::{Listener, ListeningResult},
};

// 4 for HDD 10MB, 6 for Floppy 5.25.
const ADDRESS: u8 = 5;

fn main() {
    gpio::reset_all();

    loop {
        let mut request = None;

        let mut listener = Listener::new(ADDRESS);

        'listen: loop {
            match listener.listen() {
                ListeningResult::Done { bytes } => {
                    request = Some(bytes);
                }
                ListeningResult::Unhandled { byte, is_command } if is_command => {
                    if message::is_mta(byte, ADDRESS) {
                        break 'listen;
                    } else {
                        println!("Unrecognized command {byte:#010b}");
                    }
                }
                _ => {
                    continue 'listen;
                }
            }
        }

        drop(listener);

        let request = request.and_then(parse_request);

        println!("Talking is unsupported");

        continue;
    }
}

fn parse_request(raw: Vec<u8>) -> Option<Request> {
    match Request::try_from(raw.as_slice()) {
        Ok(value) => {
            println!("Successfully parse {value:?}");
            Some(value)
        },
        Err(err) => {
            println!("Failed to parse request {raw:02x?}: {err}");
            None
        }
    }
}
