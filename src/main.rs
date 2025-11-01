mod gpib;
mod gpio;
mod listener;
mod messages;

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use rppal::gpio::{Level, Result};

use crate::{gpib::GPIB, listener::ListenerStateMachine};

// 4 for HDD 10MB, 6 for Floppy 5.25.
const ADDRESS: u8 = 5;

fn main() -> Result<()> {
    let start = Instant::now();

    gpio::reset_all()?;

    let _dc = gpio::output(GPIB::DC, Level::High)?;
    let _te = gpio::output(GPIB::TE, Level::Low)?;
    let _pe = gpio::output(GPIB::PE, Level::Low)?;

    let atn = gpio::input(GPIB::ATN)?;
    let eoi = gpio::input(GPIB::EOI)?;

    let dav = gpio::input(GPIB::DAV)?;
    let mut _srq = gpio::output(GPIB::SRQ, Level::High)?;
    let mut ndac = gpio::output(GPIB::NDAC, Level::Low)?;
    let mut nrfd = gpio::output(GPIB::NRFD, Level::Low)?;

    let data = GPIB::data().map(|gpib| gpio::input(gpib).unwrap());

    let mut listener = ListenerStateMachine::new(ADDRESS);

    loop {
        // Ready for data.
        ndac.set_low();
        nrfd.set_high();

        // Wait laptop.
        // println!("Wait DAV low");
        while dav.is_high() {
            sleep(Duration::from_micros(100));
        }
        // println!("DAV is low");

        // Not ready for data.
        nrfd.set_low();

        // Read all.
        let atn = atn.is_low() as u8;
        let eoi = eoi.is_low() as u8;
        let byte = gpio::read_data(&data)?;

        println!(
            "{}ms GPIB: ATN={atn} EOI={eoi} BYTE={byte:#02x} ({byte:#08b})",
            (Instant::now() - start).as_millis()
        );

        let received = listener.process(byte, atn == 1);
        if let Some(received) = received {
            println!("Listener completed: {received:02x?}");
        }

        // Notify that we read byte.
        ndac.set_high();

        // Wait laptop.
        // println!("Wait DAV high");
        while dav.is_low() {
            sleep(Duration::from_micros(100));
        }
        // println!("DAV is high");
    }
}
