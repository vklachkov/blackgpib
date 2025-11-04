use std::time::Duration;

use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpib_command::GPIBCommand, gpio, utils::busy_wait};

#[derive(Debug, PartialEq)]
pub enum ListeningResult {
    /// Byte successfully received and processed.
    Continue,

    // Received listen for another target
    AnotherTarget,

    /// Data reading finished; the device returned to the MLA waiting state.
    Done {
        bytes: Vec<u8>,
    },

    /// A byte was read that cannot be interpreted in the listener's current state.
    /// This could be a command (for example, DSL) or a byte meant for another device.
    UnhandledCommand {
        cmd: GPIBCommand,
    },
}

/// Listener represents device in Listen state.
/// Allow just read bytes and nothing more.
#[allow(unused)]
pub struct Listener {
    state_machine: StateMachine,

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

    // is set when controller asks to listen not on our address
    set_error: bool,
}

impl Listener {
    pub fn new(address: u8) -> Self {
        Self {
            state_machine: StateMachine::new(address),

            dc: gpio::output(GPIB::DC, Level::High),
            te: gpio::output(GPIB::TE, Level::Low),
            pe: gpio::output(GPIB::PE, Level::High),

            atn: gpio::input(GPIB::ATN),
            srq: gpio::output(GPIB::SRQ, Level::High),
            ren: gpio::input(GPIB::REN),
            ifc: gpio::input(GPIB::IFC),
            eoi: gpio::input(GPIB::EOI),
            dav: gpio::input(GPIB::DAV),
            ndac: gpio::output(GPIB::NDAC, Level::Low),
            nrfd: gpio::output(GPIB::NRFD, Level::Low),

            data: GPIB::data().map(gpio::input),

            set_error: false,
        }
    }

    pub fn reset(&mut self) {
        self.state_machine.reset();
    }

    /// Implements a full handshake cycle as described in the standard
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
    pub fn listen(&mut self) -> ListeningResult {
        // Ready for a new byte.
        if !self.set_error {
            self.ndac.set_low();
        }
        self.nrfd.set_high();

        if self.set_error {
            log::debug!("Set Error state");
            busy_wait(Duration::from_micros(15));
            self.set_error = false;
            self.ndac.set_low();

            log::debug!("Wait ATN High");
            while self.atn.read() != Level::High {}
            self.ndac.set_high();
            log::debug!("Wait ATN Low");
            while self.atn.read() != Level::Low {}
            self.ndac.set_low();
            log::debug!("Data skipped");
        }

        // Wait until Compass sets the data on the bus and raise the DAta Valid flag.
        while self.dav.read() != Level::Low {}

        // Not ready to receive a new byte, reading in progress.
        self.nrfd.set_low();

        // Read byte and flags.
        let atn = self.atn.is_low() as u8;
        // let eoi = self.eoi.is_low() as u8;
        let byte = gpio::read_data(&self.data);

        // All good, now we can process the received byte without rushing.
        // The laptop will wait until we say we're ready for new data.
        // log::debug!("ATN={atn} EOI={eoi} BYTE={byte:#02x} ({byte:#08b})");

        let ret = self.state_machine.process(byte, atn == 1);
        if ret == ListeningResult::AnotherTarget {
            log::debug!("state = {ret:?}");
            self.set_error = true;
        }

        // Signal that we've read the byte.
        self.ndac.set_high();

        // Wait until the laptop resets the DAta Valid flag.
        while self.dav.read() != Level::High {}

        busy_wait(Duration::from_micros(10));

        return ret;
    }

    pub fn srq_low(&mut self) {
        self.srq.set_low();
    }
}

/// Listener state machine, implements
/// 2.6.2 L Function State Diagram.
struct StateMachine {
    /// Device address for correct parsing of MLA command.
    dev_address: u8,

    /// Represents `LIDS` (if false) and `LACS` (if true) state
    /// in terms of the standard.
    is_active: bool,

    /// Buffer for all bytes after MLA for our device.
    buffer: Vec<u8>,
}

impl StateMachine {
    fn new(dev_address: u8) -> Self {
        Self {
            dev_address,
            is_active: false,
            buffer: Vec::with_capacity(1024),
        }
    }

    fn reset(&mut self) {
        self.is_active = false;
        self.buffer.clear();
    }

    fn process(&mut self, byte: u8, is_command: bool) -> ListeningResult {
        if self.is_active {
            self.process_active_byte(byte, is_command)
        } else {
            self.process_idle_byte(byte, is_command)
        }
    }

    fn process_active_byte(&mut self, byte: u8, is_command: bool) -> ListeningResult {
        if is_command {
            let cmd = GPIBCommand::from(byte);
            if cmd == GPIBCommand::UNL {
                // log::debug!("UNL received");

                self.is_active = false;

                let bytes = self.buffer.clone();
                self.buffer.clear();

                ListeningResult::Done { bytes }
            } else if cmd == GPIBCommand::DCL {
                // log::debug!("DCL received");

                self.is_active = false;
                self.buffer.clear();
                ListeningResult::Continue
            } else {
                ListeningResult::UnhandledCommand { cmd }
            }
        } else {
            // log::debug!("...add `{byte:#04x}` to buffer");
            self.buffer.push(byte);

            ListeningResult::Continue
        }
    }

    fn process_idle_byte(&mut self, byte: u8, is_command: bool) -> ListeningResult {
        if !is_command {
            return ListeningResult::Continue;
        }

        match GPIBCommand::from(byte) {
            GPIBCommand::MLA(address) if address == self.dev_address => {
                self.is_active = true;
                ListeningResult::Continue
            }
            GPIBCommand::MLA(_) => ListeningResult::AnotherTarget,
            cmd => ListeningResult::UnhandledCommand { cmd },
        }
    }
}
