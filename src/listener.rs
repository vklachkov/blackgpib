use std::{fmt::Debug, ops::Deref};

use crate::{
    common::CommonPins,
    gpib_command::GPIBCommand,
    gpib_pinout::GPIBPin,
    rppal::{Gpio, InputPin, Level, OutputPin},
};

#[allow(unused)]
pub struct Listener<'gpio> {
    common: &'gpio CommonPins<'gpio>,

    eoi: InputPin<'gpio>,
    dav: InputPin<'gpio>,

    ndac: OutputPin<'gpio>,
    nrfd: OutputPin<'gpio>,

    data: [InputPin<'gpio>; 8],
}

#[derive(Clone, Copy, Debug)]
pub struct Byte {
    pub value: u8,
    pub atn: bool,
    pub eoi: bool,
}

impl<'gpio> Listener<'gpio> {
    pub fn new(gpio: &'gpio Gpio, common: &'gpio CommonPins<'gpio>) -> Self {
        Self {
            common,

            eoi: unsafe { gpio.get(GPIBPin::EOI.pin_number()) }.into_input_pullup(),
            dav: unsafe { gpio.get(GPIBPin::DAV.pin_number()) }.into_input_pullup(),

            ndac: unsafe { gpio.get(GPIBPin::NDAC.pin_number()) }.into_output_high(),
            nrfd: unsafe { gpio.get(GPIBPin::NRFD.pin_number()) }.into_output_high(),

            data: GPIBPin::data().map(|gpib_pin| {
                //
                unsafe { gpio.get(gpib_pin.pin_number()) }.into_input_pullup()
            }),
        }
    }

    fn read_byte(&self) -> Byte {
        let atn = self.common.atn.is_low();
        let eoi = self.eoi.is_low();
        let value = self.read_data();

        return Byte { atn, eoi, value };
    }

    #[rustfmt::skip]
    fn read_data(&self) -> u8 {
        (self.data[0].is_low() as u8) << 0 |
        (self.data[1].is_low() as u8) << 1 |
        (self.data[2].is_low() as u8) << 2 |
        (self.data[3].is_low() as u8) << 3 |
        (self.data[4].is_low() as u8) << 4 |
        (self.data[5].is_low() as u8) << 5 |
        (self.data[6].is_low() as u8) << 6 |
        (self.data[7].is_low() as u8) << 7
    }

    /// Waits valid data on GPiB bus and reads byte without handshake.
    pub fn sniff_byte(&self) -> Byte {
        // Wait until Compass sets the data on the bus.
        while self.dav.read() != Level::Low {}

        let byte = self.read_byte();

        // Wait until all devices read the byte.
        // The Raspberry Pi is too fast, so if we don't wait until the flag is reset,
        // the sniffer can read the same byte several times (if function used in a loop).
        while self.dav.read() != Level::High {}

        return byte;
    }

    pub fn start_command_handshake<'l>(&'l self) -> HandshakeGuard<'l, 'gpio, GPIBCommand> {
        loop {
            if self.common.atn.read() != Level::Low {
                self.ndac.set_high();
                continue;
            }

            self.ndac.set_low();

            if self.dav.read() != Level::Low {
                continue;
            }

            self.nrfd.set_low();

            let byte = self.read_byte();
            let cmd = GPIBCommand::from(byte.value);

            break self.handshake_guard(cmd);
        }
    }

    pub fn start_data_handshake<'l>(&'l self) -> HandshakeGuard<'l, 'gpio, Byte> {
        while self.dav.read() != Level::Low {}

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
        self.nrfd.set_low();
        self.ndac.set_high();

        while self.dav.read() != Level::High {}

        self.ndac.set_low();
        self.nrfd.set_high();
    }

    fn unexpected_data_received(&self) {
        self.ndac.set_high();

        while self.dav.read() != Level::High {}

        self.nrfd.set_high();
    }

    /// Raise SRQ pin.
    pub fn service_request(&self) {
        self.common.srq.set_low();
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
