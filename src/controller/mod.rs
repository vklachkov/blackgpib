mod disk_request;

// use crate::{gpib_command::GPIBCommand, listener::Listener, talker::Talker};

use disk_request as request;

pub struct DeviceController {
    address: u8,
}

impl DeviceController {
    pub fn new(address: u8) -> Self {
        Self { address }
    }

    pub fn start(self) {
        // TODO
    }
}
