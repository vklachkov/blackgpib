use std::time::{Duration, Instant};

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpio};

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

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        for i in 0..bytes.len() {
            println!("Start sending byte {i} ({:#04x})", bytes[i]);

            self.dav.set_high();

            if self.ndac.is_high() && self.nrfd.is_high() {
                unimplemented!("NDAC=high NRFD=high");
            }

            gpio::write_data(&mut self.data, bytes[i]);

            if i == bytes.len() - 1 {
                println!("EOI low");
                self.eoi.set_low();
            } else {
                self.eoi.set_high();
            }

            Self::busy_wait(Duration::from_micros(10));

            while self.nrfd.read() != Level::High {}

            self.dav.set_low();

            while self.ndac.read() != Level::High {}

            self.dav.set_high();

            Self::busy_wait(Duration::from_micros(20));
        }
    }

    fn busy_wait(duration: Duration) {
        let start = Instant::now();
        while Instant::now() - start < duration {}
    }
}
