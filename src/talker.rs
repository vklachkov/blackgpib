#![allow(unused)]

use std::time::Duration;

use crate::{
    common::CommonPins,
    gpib_command::GPIBCommand,
    gpib_pinout::GPIBPin,
    gpio::{Gpio, InputPin, Level, OutputPin},
    trace,
    utils::busy_wait,
};

#[allow(unused)]
pub struct Talker<'gpio, 'p> {
    common: &'p CommonPins<'gpio>,

    eoi: OutputPin<'gpio>,
    dav: OutputPin<'gpio>,

    ndac: InputPin<'gpio>,
    nrfd: InputPin<'gpio>,

    data: [OutputPin<'gpio>; 8],
}

impl<'gpio, 'p> Talker<'gpio, 'p> {
    pub const INTERBYTE_DELAY: Duration = Duration::from_micros(25);

    pub fn new(gpio: &'gpio Gpio, common: &'p CommonPins<'gpio>) -> Self {
        Self {
            common,

            eoi: unsafe { gpio.get(GPIBPin::EOI.pin_number()) }.into_output_high(),
            dav: unsafe { gpio.get(GPIBPin::DAV.pin_number()) }.into_output_high(),

            ndac: unsafe { gpio.get(GPIBPin::NDAC.pin_number()) }.into_input_pullup(),
            nrfd: unsafe { gpio.get(GPIBPin::NRFD.pin_number()) }.into_input_pullup(),

            data: GPIBPin::data().map(|gpib_pin| {
                //
                unsafe { gpio.get(gpib_pin.pin_number()) }.into_output_high()
            }),
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
                busy_wait(Self::INTERBYTE_DELAY.as_nanos() as _);
            }
        }

        crate::info!("End transmition");
    }

    pub fn send_serial_poll_response(&self, byte: u8) {
        crate::info!("Send serial poll response {:#04x} to bus", byte);

        self.wait_atn(Level::High);

        self.send_byte(byte, false);

        crate::info!("End transmition");
    }

    #[inline]
    fn wait_atn(&self, level: Level) {
        while self.common.atn.read() != level {}
    }

    fn send_byte(&self, byte: u8, eoi: bool) {
        if eoi {
            self.eoi.write(Level::Low);
        }

        self.write_data(byte);
        busy_wait(Duration::from_micros(10).as_nanos() as _);

        // TODO
        while self.ndac.read() != Level::Low || self.nrfd.read() != Level::High {}

        // Now we can signal that data is valid.
        self.dav.set_low();

        // Wait until the laptop signals successful data read.
        while self.ndac.read() != Level::High {}

        // Signal that data is no longer valid.
        self.dav.set_high();

        if eoi {
            busy_wait(Duration::from_micros(20).as_nanos() as _);
            self.eoi.write(Level::High);
        }
    }

    fn write_data(&self, byte: u8) {
        self.data[0].write(!Level::from((byte >> 0) & 1));
        self.data[1].write(!Level::from((byte >> 1) & 1));
        self.data[2].write(!Level::from((byte >> 2) & 1));
        self.data[3].write(!Level::from((byte >> 3) & 1));
        self.data[4].write(!Level::from((byte >> 4) & 1));
        self.data[5].write(!Level::from((byte >> 5) & 1));
        self.data[6].write(!Level::from((byte >> 6) & 1));
        self.data[7].write(!Level::from((byte >> 7) & 1));
    }
}
