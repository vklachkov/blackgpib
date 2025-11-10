use crate::{devices::Device, talker::Talker};

pub struct GenericPrinter {}

impl GenericPrinter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Device for GenericPrinter {
    fn reset(&mut self) {
        // todo!()
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool {
        // todo!()
        false
    }

    fn talk(&mut self, talker: Talker) {
        // todo!()
    }
}
