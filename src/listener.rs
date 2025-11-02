use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpio, message};

pub enum ListeningResult {
    /// Byte successfully received and processed.
    Continue,

    /// Data reading finished; the device returned to the MLA waiting state.
    Done { bytes: Vec<u8> },

    /// A byte was read that cannot be interpreted in the listener's current state.
    /// This could be a command (for example, DSL) or a byte meant for another device.
    Unhandled { byte: u8, is_command: bool },
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
}

impl Listener {
    pub fn new(address: u8) -> Self {
        Self {
            state_machine: StateMachine::new(address),

            dc: gpio::output(GPIB::DC, Level::High),
            te: gpio::output(GPIB::DC, Level::Low),
            pe: gpio::output(GPIB::DC, Level::High),

            atn: gpio::input(GPIB::ATN),
            srq: gpio::output(GPIB::SRQ, Level::High),
            ren: gpio::input(GPIB::REN),
            ifc: gpio::input(GPIB::IFC),
            eoi: gpio::input(GPIB::EOI),
            dav: gpio::input(GPIB::DAV),
            ndac: gpio::output(GPIB::NDAC, Level::Low),
            nrfd: gpio::output(GPIB::NRFD, Level::Low),

            data: GPIB::data().map(gpio::input),
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
        self.ndac.set_low();
        self.nrfd.set_high();

        // Wait until Compass sets the data on the bus and raise the DAta Valid flag.
        while self.dav.read() != Level::Low {}

        // Not ready to receive a new byte, reading in progress.
        self.nrfd.set_low();

        // Read byte and flags.
        let atn = self.atn.is_low() as u8;
        let eoi = self.eoi.is_low() as u8;
        let byte = gpio::read_data(&self.data);

        // Signal that we've read the byte.
        self.ndac.set_high();

        // Wait until the laptop resets the DAta Valid flag.
        while self.dav.read() != Level::High {}

        // All good, now we can process the received byte without rushing.
        // The laptop will wait until we say we're ready for new data.
        println!("ATN={atn} EOI={eoi} BYTE={byte:#02x} ({byte:#08b})");

        self.state_machine.process(byte, atn == 1)
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
            buffer: Vec::with_capacity(10),
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
            if message::is_unl(byte) {
                self.is_active = false;

                let bytes = self.buffer.clone();
                self.buffer.clear();

                ListeningResult::Done { bytes }
            } else {
                ListeningResult::Unhandled { byte, is_command }
            }
        } else {
            println!("Add `{byte:#04x}` to buffer");
            self.buffer.push(byte);

            ListeningResult::Continue
        }
    }

    fn process_idle_byte(&mut self, byte: u8, is_command: bool) -> ListeningResult {
        if is_command && message::is_mla(byte, self.dev_address) {
            self.is_active = true;
            ListeningResult::Continue
        } else {
            ListeningResult::Unhandled { is_command, byte }
        }
    }
}
