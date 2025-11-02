mod disk_request;
mod gpib;
mod gpio;
mod listener;
mod message;
mod talker;

use std::{
    fs,
    sync::LazyLock,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    disk_request::{Request, RequestCode},
    listener::{Listener, ListeningResult},
    talker::Talker,
};

static START: LazyLock<Instant> = LazyLock::new(|| Instant::now());

// 4 for HDD 10MB, 5/6 for Floppy 5.25.
const ADDRESS: u8 = 4;

// TODO: Represent as struct not bytes.
// Description here http://deltacxx.insomnia247.nl/projects/gridcompass/disk_info.txt.
const IDENTITY: [u8; 52] = [
    0x00, 0x02, 0xf8, 0x01, 0xd0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38,
    0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4c, 0x4f, 0x50, 0x50,
    0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x30, 0x32, 0x33, 0x37, 0x2d, 0x30, 0x30, 0x00, 0x02,
    0x09, 0x00, 0x02, 0x00,
];

fn main() {
    gpio::reset_all();

    let image = fs::read("GRIDOS.img").unwrap();

    let mut sector = 0;
    let mut size = 0;

    let start = *START;

    loop {
        let result = listen();

        println!("{}ms Init talker...", (Instant::now() - *START).as_millis());

        let mut talker = Talker::new();

        println!(
            "{}ms ATN must be high...",
            (Instant::now() - *START).as_millis()
        );

        let Some((request, serial_poll)) = result.and_then(|(r, s)| Some((parse_request(r)?, s)))
        else {
            println!("Sending part of data");
            talker.send_bytes(&image[(sector * 512)..(sector * 512 + size)]);
            println!("Success");
            continue;
        };

        println!("Request from Compass: {request:?}");

        let response: &[u8] = match request.code {
            RequestCode::GetStatus => &IDENTITY,
            RequestCode::Read => {
                // TODO: Check sector
                // Return sector
                sector = request.sector_number as usize;
                size = request.data_size as usize;
                println!("Yeeeeeeeee, we must read {} bytes", request.data_size);
                &[0x4f]
            }
            RequestCode::Write => {
                // Patch image
                unimplemented!()
            }
            _ => {
                unimplemented!("Unsupported command");
            }
        };

        talker.send_bytes(response);

        println!("Bytes sent!");
    }
}

fn listen() -> Option<(Vec<u8>, bool)> {
    let mut listener = Listener::new(ADDRESS);
    let mut request = None;
    let mut serial_poll = false;

    loop {
        match listener.listen() {
            ListeningResult::Done { bytes } => {
                println!("Bytes {bytes:02x?}! But we must continue to listen");
                // Hack!
                if bytes.get(0) == Some(&4) {
                    listener.srq_low();
                }
                request = Some(bytes);
            }
            ListeningResult::Unhandled { byte, is_command } if is_command => {
                if message::is_mta(byte, ADDRESS) {
                    println!(
                        "{}ms MTA received, drop listener",
                        (Instant::now() - *START).as_millis()
                    );
                    listener.srq_high();
                    let a = request.map(|r| (r, serial_poll));
                    return a;
                } else if message::is_spe(byte) {
                    println!("Serial Poll Enable");
                    serial_poll = true;
                } else if message::is_spd(byte) {
                    println!("Serial Poll Disable");
                    serial_poll = false;
                } else if message::is_dcl(byte) {
                    println!("Device CLear received");
                    // listener.reset();
                } else if message::is_unt(byte) {
                    println!("Untalk received")
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
