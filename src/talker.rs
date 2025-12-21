#![allow(unused)]

use std::time::Duration;

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{
    gpib_command::GPIBCommand,
    gpib_gpio,
    gpib_pinout::GPIBPin,
    trace,
    utils::busy_wait,
};

#[allow(unused)]
pub struct Talker {
    dc: OutputPin,
    te: OutputPin,
    pe: OutputPin,

    atn: InputPin,
    srq: OutputPin,
    ren: InputPin,
    ifc: InputPin,
    eoi: OutputPin,
    dav: OutputPin,

    ndac: InputPin,
    nrfd: InputPin,

    data: [OutputPin; 8],
}

impl Talker {
    pub const INTERBYTE_DELAY: Duration = Duration::from_micros(25);

    pub fn new() -> Self {
        Self {
            dc: gpib_gpio::output(GPIBPin::DC, Level::High),
            te: gpib_gpio::output(GPIBPin::TE, Level::High),
            pe: gpib_gpio::output(GPIBPin::PE, Level::High),

            atn: gpib_gpio::input(GPIBPin::ATN),
            srq: gpib_gpio::output(GPIBPin::SRQ, Level::High),
            ren: gpib_gpio::input(GPIBPin::REN),
            ifc: gpib_gpio::input(GPIBPin::IFC),
            eoi: gpib_gpio::output(GPIBPin::EOI, Level::High),
            dav: gpib_gpio::output(GPIBPin::DAV, Level::High),

            ndac: gpib_gpio::input(GPIBPin::NDAC),
            nrfd: gpib_gpio::input(GPIBPin::NRFD),

            data: GPIBPin::data().map(|pin| gpib_gpio::output(pin, Level::High)),
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        let bytes_len = bytes.len();

        crate::info!("Send {bytes_len} bytes to bus... Wait ATN");

        self.wait_atn(Level::High);

        crate::info!("ATN high, start transmition");

        for i in 0..bytes_len {
            let eoi = i == bytes_len - 1;
            self.send_byte(bytes[i], eoi);
            if !eoi {
                busy_wait(Self::INTERBYTE_DELAY);
            }
        }

        crate::info!("End transmition");
    }

    pub fn send_serial_poll_response(&mut self, byte: u8) {
        crate::info!("Send serial poll response {:#04x} to bus", byte);

        self.wait_atn(Level::High);

        self.send_byte(byte, false);

        crate::info!("End transmition");
    }

    #[inline]
    fn wait_atn(&self, level: Level) {
        while self.atn.read() != level {}
    }

    fn send_byte(&mut self, byte: u8, eoi: bool) {
        if eoi {
            self.eoi.write(Level::Low);
        }

        gpib_gpio::write_data(&mut self.data, byte);
        busy_wait(Duration::from_micros(10));

        // TODO
        while self.ndac.read() != Level::Low || self.nrfd.read() != Level::High {}

        // Now we can signal that data is valid.
        self.dav.set_low();

        // Wait until the laptop signals successful data read.
        while self.ndac.read() != Level::High {}

        // Signal that data is no longer valid.
        self.dav.set_high();

        if eoi {
            busy_wait(Duration::from_micros(20));
            self.eoi.write(Level::High);
        }
    }
}
