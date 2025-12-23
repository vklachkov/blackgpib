mod mem;
mod mode;
mod pin;
mod pinout;
mod system;
mod types;

use std::io;

use mem::GpioMem;

pub use mode::{listener::GpioMode as ListenerGpio, talker::GpioMode as TalkerGpio};
pub use pin::{InputPin, OutputPin};

#[derive(Debug)]
pub struct Gpio {
    mem: GpioMem,
}

impl Gpio {
    pub unsafe fn new() -> io::Result<Gpio> {
        Ok(Self { mem: GpioMem::open()? })
    }

    pub fn into_listener_mode(&mut self) -> ListenerGpio<'_> {
        ListenerGpio::new(&mut self.mem)
    }

    pub fn into_talker_mode(&mut self) -> TalkerGpio<'_> {
        TalkerGpio::new(&mut self.mem)
    }
}
