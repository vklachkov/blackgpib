use crate::devices::Device;

pub struct GenericPrinter {}

impl GenericPrinter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Device for GenericPrinter {
    fn reset(&mut self) {
        todo!()
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool {
        todo!()
    }
    
    fn talk(&mut self, talker: crate::talker::Talker) {
        todo!()
    }
}
