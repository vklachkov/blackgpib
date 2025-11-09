use std::time::Duration;

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpio, utils::busy_wait};

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
    pub const INTERBYTE_DELAY: Duration = Duration::from_micros(10);

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

    /// Sends a byte to the bus with atn flag and
    /// [`Self::INTERBYTE_DELAY`] delay after sending.
    pub fn send_command(&mut self, cmd: GPIBCommand) {
        self.send_byte(cmd.into(), true, false);

        busy_wait(Self::INTERBYTE_DELAY);
    }

    /// Sends all `bytes` to the bus with a delay [`Self::INTERBYTE_DELAY`] between bytes.
    /// For the last byte, the eoi flag will be set if the `send_eoi` flag is true.
    pub fn send_bytes(&mut self, bytes: &[u8], send_eoi: bool) {
        for i in 0..bytes.len() {
            let is_last_byte = i == bytes.len() - 1;

            self.send_byte(bytes[i], false, is_last_byte && send_eoi);

            busy_wait(Self::INTERBYTE_DELAY);
        }
    }

    /// Send byte in bus with a full handshake cycle as described in the standard
    /// in section "Annex B Handshake Process Timing Sequence".
    pub fn send_byte(&mut self, byte: u8, atn: bool, eoi: bool) {
        // Notify whether the next byte on the bus will be a command or not.
        self.atn.write(if atn { Level::Low } else { Level::High });

        // Notify that data on the bus is no longer valid.
        self.dav.set_high();

        // Wait laptop readiness.
        if self.ndac.is_high() && self.nrfd.is_high() {
            while self.ndac.is_high() && self.nrfd.is_high() {}
        }

        // Write data to the bus and set the last byte flag.
        self.eoi.write(if eoi { Level::Low } else { Level::High });
        gpib_gpio::write_data(&mut self.data, byte);

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
    }
}
