mod gpib;
mod gpio;

use std::{thread::sleep, time::Duration};

use rppal::gpio::{Level, Result};

use crate::gpib::GPIB;

fn main() -> Result<()> {
    gpio::reset_all()?;

    let _dc = gpio::output(GPIB::DC, Level::High)?;
    let _te = gpio::output(GPIB::TE, Level::Low)?;
    let _pe = gpio::output(GPIB::PE, Level::High)?;

    let atn = gpio::input(GPIB::ATN)?;
    let eoi = gpio::input(GPIB::EOI)?; 

    let dav = gpio::input(GPIB::DAV)?;
    let mut ndac = gpio::output(GPIB::NDAC, Level::Low)?;
    let mut nrfd = gpio::output(GPIB::NRFD, Level::Low)?;

    sleep(Duration::from_millis(500));

    loop {
        // Ready for data.
        ndac.set_low();
        nrfd.set_high();

        // Wait laptop.
        println!("Wait DAV low");
        while dav.is_high() {
            sleep(Duration::from_micros(100));
        }
        println!("DAV is low");

        // Not ready for data.
        nrfd.set_low();

        // Read all.
        let is_atn = atn.is_low();
        let is_eoi = eoi.is_low();
        let byte = gpio::read_data()?;

        println!("GPIB -> ATN={is_atn} EOI={is_eoi} BYTE={byte:2x}");

        // Notify that we read byte.
        ndac.set_high();

        // Wait laptop.
        println!("Wait DAV high");
        while dav.is_low() {
            sleep(Duration::from_micros(100));
        }
        println!("DAV is high");
    }
}
