use std::{fmt::Debug, ops::Deref};

use crate::gpio;

use super::Command;

#[derive(Clone, Copy, Debug)]
pub struct Byte {
    pub value: u8,
    pub atn: bool,
    pub eoi: bool,
}

#[allow(unused)]
pub struct Listener<'gpio> {
    gpio: gpio::ListenerGpio<'gpio>,
}

impl<'gpio> Listener<'gpio> {
    pub fn new(gpio: &'gpio mut gpio::Gpio) -> Self {
        Self {
            gpio: gpio.into_listener_mode(),
        }
    }

    fn read_byte(&self) -> Byte {
        let atn = self.gpio.atn().is_low();
        let eoi = self.gpio.eoi().is_low();
        let value = self.gpio.read_dio();

        return Byte { atn, eoi, value };
    }

    /// Waits valid data on GPIB bus and reads byte without handshake.
    pub fn sniff_byte(&self) -> Byte {
        // Wait until Compass sets the data on the bus.
        while self.gpio.dav().is_high() {}

        let byte = self.read_byte();

        // Wait until all devices read the byte.
        // The Raspberry Pi is too fast, so if we don't wait until the flag is reset,
        // the sniffer can read the same byte several times (if function used in a loop).
        while self.gpio.dav().is_low() {}

        return byte;
    }

    pub fn start_command_handshake<'l>(&'l self) -> HandshakeGuard<'l, 'gpio, Command> {
        loop {
            if self.gpio.atn().is_high() {
                self.gpio.ndac().set_high();
                continue;
            }

            self.gpio.ndac().set_low();

            if self.gpio.dav().is_high() {
                continue;
            }

            self.gpio.nrfd().set_low();

            let byte = self.read_byte();
            let cmd = Command::from(byte.value);

            break self.handshake_guard(cmd);
        }
    }

    pub fn start_data_handshake<'l>(&'l self) -> HandshakeGuard<'l, 'gpio, Byte> {
        while self.gpio.dav().is_high() {}

        let byte = self.read_byte();

        return self.handshake_guard(byte);
    }

    fn handshake_guard<'l, T>(&'l self, value: T) -> HandshakeGuard<'l, 'gpio, T> {
        HandshakeGuard {
            listener: self,
            value,
            unexpected: false,
        }
    }

    fn end_handshake(&self) {
        self.gpio.nrfd().set_low();
        self.gpio.ndac().set_high();

        while self.gpio.dav().is_low() {}

        self.gpio.ndac().set_low();
        self.gpio.nrfd().set_high();
    }

    fn unexpected_data_received(&self) {
        self.gpio.ndac().set_high();

        while self.gpio.dav().is_low() {}

        self.gpio.nrfd().set_high();
    }

    /// Raise SRQ pin.
    pub fn service_request(&self) {
        self.gpio.srq().set_low();
    }
}

pub struct HandshakeGuard<'l, 'p: 'l, T> {
    listener: &'l Listener<'p>,
    value: T,
    unexpected: bool,
}

impl<T> HandshakeGuard<'_, '_, T> {
    pub fn expected(mut self) {
        self.unexpected = false;
    }

    pub fn unexpected(mut self) {
        self.unexpected = true;
    }
}

impl<T: Debug> Debug for HandshakeGuard<'_, '_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> Deref for HandshakeGuard<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Drop for HandshakeGuard<'_, '_, T> {
    fn drop(&mut self) {
        if self.unexpected {
            crate::trace!("unexpected byte, wait next");
            self.listener.unexpected_data_received();
        } else {
            crate::trace!("expected byte");
            self.listener.end_handshake();
        }
    }
}
