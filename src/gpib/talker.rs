use std::time::Duration;

use crate::{gpib::Command, gpio, time_utils::busy_wait, trace};

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
        self.gpio.atn().set_high();

        let bytes_len = bytes.len();
        trace!("Send {bytes_len} bytes");

        for i in 0..bytes_len {
            let eoi = i == bytes_len - 1;
            self.send_byte(bytes[i], eoi);
            busy_wait(Self::INTERBYTE_DELAY);
        }
    }

    pub fn send_serial_poll_response(&self, byte: u8) {
        trace!("Send {byte:#04x} as SRQ response");
        self.send_byte(byte, false);
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

    pub fn send_command(&self, command: Command) {
        trace!("Send command {command:?}");

        self.gpio.atn().set_low();

        trace!("Wait NDAC=low");
        while self.gpio.ndac().is_high() {}

        self.gpio.write_dio(command.into());
        busy_wait(Self::DIO_SETTLE_DELAY);

        self.gpio.dav().set_low();

        trace!("Wait NDAC=high");
        while self.gpio.ndac().is_low() {}

        self.gpio.dav().set_high();
    }

    pub fn wait_srq(&mut self) {
        trace!("Wait SRQ=low");
        while self.gpio.srq().is_high() {}
    }
}
