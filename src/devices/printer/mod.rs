use crate::talker::Talker;

use super::device::{Device, ServiceRequest};

pub struct GenericPrinter {}

impl GenericPrinter {
    pub fn new() -> Self {
        Self {}
    }

    fn save_byte(&mut self, _byte: u8) {
        // TODO: Save to file or some memory buffer
    }
}

impl Device for GenericPrinter {
    fn reset(&mut self) {
        // Do nothing. The printer has no state.
    }

    fn process_byte(&mut self, byte: u8, _eoi: bool) -> ServiceRequest {
        self.save_byte(byte);
        ServiceRequest::NotRequired
    }

    fn talk(&mut self, _talker: Talker) {
        // Do nothing. The printer cannot talk.
    }
}
