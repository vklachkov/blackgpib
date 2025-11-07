use std::time::Duration;

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpio, utils::busy_wait};

/// Talker represents device in Talk state.
/// Allow just send bytes and nothing more.
#[allow(unused)]
pub struct Talker {
    dc: OutputPin,
    te: OutputPin,
    pe: OutputPin,

    atn: OutputPin,
    srq: InputPin,
    ren: OutputPin,
    ifc: OutputPin,
    eoi: OutputPin,
    dav: OutputPin,

    ndac: InputPin,
    nrfd: InputPin,

    data: [OutputPin; 8],
}

impl Talker {
    pub fn new() -> Self {
        Self {
            dc: gpio::output(GPIB::DC, Level::Low),
            te: gpio::output(GPIB::TE, Level::High),
            pe: gpio::output(GPIB::PE, Level::High),

            atn: gpio::output(GPIB::ATN, Level::High),
            srq: gpio::input(GPIB::SRQ),
            ren: gpio::output(GPIB::REN, Level::High),
            ifc: gpio::output(GPIB::IFC, Level::High),
            eoi: gpio::output(GPIB::EOI, Level::High),
            dav: gpio::output(GPIB::DAV, Level::High),

            ndac: gpio::input(GPIB::NDAC),
            nrfd: gpio::input(GPIB::NRFD),

            data: GPIB::data().map(|pin| gpio::output(pin, Level::High)),
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8], send_eoi: bool) {
        // log::info!("Send bytes (with eoi? {send_eoi}) {}", bytes.len());
        for i in 0..bytes.len() {
            let is_last_byte = i == bytes.len() - 1;

            // println!("byte={:#04x} last={}", bytes[i], is_last_byte);

            // Notify that data on the bus is no longer valid.
            self.dav.set_high();

            // FIXME:
            // How does the real floppy drive behave when it sees such
            // a situation on the bus after Talk command?
            if self.ndac.is_high() && self.nrfd.is_high() {
                // log::debug!("NDAC=high NRFD=high when sending byte {i}");
                while self.ndac.is_high() && self.nrfd.is_high() {}
            }

            if send_eoi && is_last_byte {
                self.eoi.set_low();
            }

            // Write data to the bus and set the last byte flag.
            gpio::write_data(&mut self.data, bytes[i]);

            // FIXME: What is the delay of the real floppy drive?
            // Delay for lines to settle.
            busy_wait(Duration::from_micros(10));

            // Wait until the laptop signals it's ready for data.
            while self.nrfd.read() != Level::High {}

            // Now we can signal that data is valid.
            self.dav.set_low();

            // Wait until the laptop signals successful data read.
            while self.ndac.read() != Level::High {}

            // Signal that data is no longer valid.
            self.dav.set_high();

            // Make sure to wait before sending the next byte so the laptop
            // has time to process what it read.
            // FIXME: What is the delay of the real floppy drive?
            busy_wait(Duration::from_micros(15));
        }
    }
}
