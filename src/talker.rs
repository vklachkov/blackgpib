use std::time::Duration;

use crate::{gpio, utils::busy_wait};

#[allow(unused)]
pub struct Talker<'gpio> {
    gpio: gpio::TalkerGpio<'gpio>,
}

impl<'gpio> Talker<'gpio> {
    pub const INTERBYTE_DELAY: Duration = Duration::from_micros(20);
    pub const DIO_SETTLE_DELAY: Duration = Duration::from_micros(10);

    pub fn new(gpio: &'gpio mut gpio::Gpio) -> Self {
        Self {
            gpio: gpio.into_talker_mode(),
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        let bytes_len = bytes.len();

        crate::info!("ATN high, start transmition");

        for i in 0..bytes_len {
            let eoi = i == bytes_len - 1;
            self.send_byte(bytes[i], eoi);
            busy_wait(Self::INTERBYTE_DELAY);
        }

        crate::info!("End transmition");
    }

    pub fn send_serial_poll_response(&self, byte: u8) {
        crate::info!("Send serial poll response {:#04x} to bus", byte);

        self.send_byte(byte, false);

        crate::info!("End transmition");
    }

    fn send_byte(&self, byte: u8, eoi: bool) {
        if eoi {
            self.gpio.eoi().set_low();
        }

        self.gpio.write_dio(byte);
        busy_wait(Self::DIO_SETTLE_DELAY);

        while self.gpio.ndac().is_high() || self.gpio.nrfd().is_low() {}

        // Now we can signal that data is valid.
        self.gpio.dav().set_low();

        // Wait until the laptop signals successful data read.
        while self.gpio.ndac().is_low() {}

        // Signal that data is no longer valid.
        self.gpio.dav().set_high();

        if eoi {
            self.gpio.eoi().set_high();
        }
    }
}
