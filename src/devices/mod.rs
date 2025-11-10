mod disk;
mod manager;
mod printer;

pub use manager::DeviceManager;

use crate::talker::Talker;

pub trait Device {
    /// Resets the device to default state.
    fn reset(&mut self);

    /// Processes a byte from the bus.
    /// 
    /// Returns a flag indicating if a service request is needed.
    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool;

    /// Someone on the bus addressed you and told you "talk".
    fn talk(&mut self, talker: Talker);
}
