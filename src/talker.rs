use rppal::gpio::{InputPin, Level, OutputPin};

use crate::{gpib::GPIB, gpio, message};

/// Talker represents device in Talk state.
/// Allow just send bytes and nothing more.
#[allow(unused)]
pub struct Talker {
    state_machine: StateMachine,

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
    pub fn new(address: u8) -> Self {
        Self {
            state_machine: StateMachine::new(address),
            
            dc: gpio::output(GPIB::DC, Level::Low),
            te: gpio::output(GPIB::DC, Level::High),
            pe: gpio::output(GPIB::DC, Level::High),

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

    fn send(&mut self, bytes: &[u8]) {
        // TODO
    }
}

struct StateMachine {
    // TODO
}

impl StateMachine {
    fn new(dev_address: u8) -> Self {
        Self {
            // TODO
        }
    }
}
