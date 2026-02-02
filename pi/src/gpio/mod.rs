mod mem;
mod mode;
mod pin;
mod pinout;
mod types;

use std::io;

use self::mem::GpioMem;

pub use self::mode::{listener::GpioMode as ListenerGpio, talker::GpioMode as TalkerGpio};
pub use self::pin::{InputPin, OutputPin};

#[derive(Debug)]
pub struct Gpio {
    mem: GpioMem,
}

impl Gpio {
    /// Opens and memory-maps GPIO on the Raspberry Pi.
    ///
    /// # Safety
    ///
    /// This function must only be called once to avoid breaking invariants during pin configuration.
    pub unsafe fn new() -> io::Result<Gpio> {
        Ok(Self { mem: GpioMem::open()? })
    }

    /// Configures pins for the GPIB listener mode.
    pub fn into_listener_mode(&mut self) -> ListenerGpio<'_> {
        ListenerGpio::new(&mut self.mem)
    }

    /// Configures pins for the GPIB talker mode.
    pub fn into_talker_mode(&mut self) -> TalkerGpio<'_> {
        TalkerGpio::new(&mut self.mem)
    }
}
