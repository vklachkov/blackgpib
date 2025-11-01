mod gpib;
mod gpio;
mod listener;
mod message;

use crate::listener::{Listener, ListeningResult};

// 4 for HDD 10MB, 6 for Floppy 5.25.
const ADDRESS: u8 = 5;

fn main() {
    gpio::reset_all();

    loop {
        let mut command = None;

        let mut listener = Listener::new(ADDRESS);

        'listen: loop {
            match listener.listen() {
                ListeningResult::Done { bytes } => {
                    command = Some(bytes);
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

        panic!("Talking is unsupported");
    }
}
