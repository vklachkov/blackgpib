use std::time::Duration;

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib_gpio, gpib_pinout::GPIBPin, info, utils::busy_wait};

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

pub struct Byte {
    pub value: u8,
    pub atn: bool,
    pub eoi: bool,
}

impl Listener {
    pub fn new() -> Self {
        Self {
            ndac: gpib_gpio::output(GPIBPin::NDAC, Level::Low),
            nrfd: gpib_gpio::output(GPIBPin::NRFD, Level::Low),

            dc: gpib_gpio::output(GPIBPin::DC, Level::High),
            te: gpib_gpio::output(GPIBPin::TE, Level::Low),
            pe: gpib_gpio::output(GPIBPin::PE, Level::Low),

            atn: gpib_gpio::input(GPIBPin::ATN),
            srq: gpib_gpio::output(GPIBPin::SRQ, Level::High),
            ren: gpib_gpio::input(GPIBPin::REN),
            ifc: gpib_gpio::input(GPIBPin::IFC),
            eoi: gpib_gpio::input(GPIBPin::EOI),
            dav: gpib_gpio::input(GPIBPin::DAV),

            data: GPIBPin::data().map(gpib_gpio::input),
        }
    }

    /// Reads byte from bus with a full handshake cycle as described in the standard
    /// in section "Annex B Handshake Process Timing Sequence".
    ///
    /// This function should be called as frequently as possible to avoid missing the last byte.
    ///
    /// Although GPiB is not timing-sensitive, the GRiD Compass has an annoying bug:
    /// when sending the last byte (byte with EOI), the laptop doesn't wait for us
    /// to read the byte (and set NDAC=false) and after about ten microseconds sets ATN,
    /// resets DAV and EOI, and starts transmitting another command.
    /// No fix found. Neither NRFD delay nor anything else helped.
    /// The only solution is to read bytes as quickly as possible.
    pub fn handshake_byte(&mut self) -> Byte {
        // Ready for a new byte.
        self.ndac.set_low();
        self.nrfd.set_high();

        // Wait until Compass sets the data on the bus and raise the DAta Valid flag.
        while self.dav.read() != Level::Low {}

        // Not ready to receive a new byte, reading in progress.
        self.nrfd.set_low();

        // Read byte and flags.
        let atn = self.atn.is_low();
        let eoi = self.eoi.is_low();
        let value = gpib_gpio::read_data(&self.data);

        // Signal that we've read the byte.
        self.ndac.set_high();

        // Wait until the laptop resets the DAta Valid flag.
        while self.dav.read() != Level::High {}

        return Byte { atn, eoi, value };
    }

    /// Waits for the next command the same way a real disk does.
    pub fn wait_next_command(&mut self) {
        info!("Wait next command...");

        self.nrfd.set_high();

        busy_wait(Duration::from_micros(15));

        self.ndac.set_low();

        while self.atn.read() != Level::High {}

        self.ndac.set_high();

        while self.atn.read() != Level::Low {}

        self.ndac.set_low();
    }

    /// Raise SRQ pin.
    pub fn service_request(&mut self) {
        self.srq.set_low();
    }
}
