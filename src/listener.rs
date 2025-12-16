use std::{fmt::Debug, ops::Deref, time::Duration};

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib_command::GPIBCommand, gpib_gpio, gpib_pinout::GPIBPin, utils::busy_wait};

#[allow(unused)]
pub struct Listener {
    dc: OutputPin,
    te: OutputPin,
    pe: OutputPin,

    atn: InputPin,
    srq: OutputPin,
    ren: InputPin,
    ifc: InputPin,
    eoi: InputPin,
    dav: InputPin,

    ndac: OutputPin,
    nrfd: OutputPin,

    data: [InputPin; 8],
}

#[derive(Clone, Copy, Debug)]
pub struct Byte {
    pub value: u8,
    pub atn: bool,
    pub eoi: bool,
}

impl Listener {
    pub fn new() -> Self {
        Self {
            ndac: gpib_gpio::output(GPIBPin::NDAC, Level::High),
            nrfd: gpib_gpio::output(GPIBPin::NRFD, Level::High),

            dc: gpib_gpio::output(GPIBPin::DC, Level::High),
            te: gpib_gpio::output(GPIBPin::TE, Level::Low),
            pe: gpib_gpio::output(GPIBPin::PE, Level::High),

            atn: gpib_gpio::input(GPIBPin::ATN),
            srq: gpib_gpio::output(GPIBPin::SRQ, Level::High),
            ren: gpib_gpio::input(GPIBPin::REN),
            ifc: gpib_gpio::input(GPIBPin::IFC),
            eoi: gpib_gpio::input(GPIBPin::EOI),
            dav: gpib_gpio::input(GPIBPin::DAV),

            data: GPIBPin::data().map(gpib_gpio::input),
        }
    }

    #[inline]
    fn read_byte(&mut self) -> Byte {
        let atn = self.atn.is_low();
        let eoi = self.eoi.is_low();
        let value = gpib_gpio::read_data(&self.data);

        return Byte { atn, eoi, value };
    }

    /// Waits valid data on GPiB bus and reads byte without handshake.
    pub fn sniff_byte(&mut self) -> Byte {
        // Wait until Compass sets the data on the bus.
        while self.dav.read() != Level::Low {}

        let byte = self.read_byte();

        // Wait until all devices read the byte.
        // The Raspberry Pi is too fast, so if we don't wait until the flag is reset,
        // the sniffer can read the same byte several times (if function used in a loop).
        while self.dav.read() != Level::High {}

        return byte;
    }

    pub fn start_command_handshake<'a>(&'a mut self) -> HandshakeGuard<'a, GPIBCommand> {
        loop {
            self.ndac.set_high();

            if self.atn.read() != Level::Low {
                continue;
            }

            self.ndac.set_low();

            if self.dav.read() != Level::Low {
                continue;
            }

            self.nrfd.set_low();

            let byte = self.read_byte();

            break self.handshake_guard(GPIBCommand::from(byte.value));
        }
    }

    pub fn start_data_handshake<'a>(&'a mut self) -> HandshakeGuard<'a, Byte> {
        while self.dav.read() != Level::Low {}

        let byte = self.read_byte();

        return self.handshake_guard(byte);
    }

    fn handshake_guard<'a, T>(&'a mut self, value: T) -> HandshakeGuard<'a, T> {
        HandshakeGuard {
            listener: self,
            value,
            unexpected: false,
        }
    }

    fn end_handshake(&mut self) {
        self.nrfd.set_low();
        self.ndac.set_high();

        while self.dav.read() != Level::High {}

        self.ndac.set_low();
        self.nrfd.set_high();
    }

    fn unexpected_data_received(&mut self) {
        self.ndac.set_high();

        while self.dav.read() != Level::High {}

        self.nrfd.set_high();
    }

    pub fn wait_atn_before_talk(mut self) {
        busy_wait(Duration::from_micros(5));

        self.nrfd.set_high();
        self.ndac.set_high();

        while self.atn.read() != Level::High {}
    }

    /// Raise SRQ pin.
    pub fn service_request(&mut self) {
        self.srq.set_low();
    }
}

pub struct HandshakeGuard<'a, T> {
    listener: &'a mut Listener,
    value: T,
    unexpected: bool,
}

impl<'a, T> HandshakeGuard<'a, T> {
    pub fn expected(mut self) {
        self.unexpected = false;
    }

    pub fn unexpected(mut self) {
        self.unexpected = true;
    }
}

impl<'a, T: Debug> Debug for HandshakeGuard<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<'a, T> Deref for HandshakeGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> Drop for HandshakeGuard<'a, T> {
    fn drop(&mut self) {
        if self.unexpected {
            // crate::trace!("unexpected byte, wait next");
            self.listener.unexpected_data_received();
        } else {
            // crate::trace!("expected byte");
            self.listener.end_handshake();
        }
    }
}
