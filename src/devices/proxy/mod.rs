use std::{io::Write, os::unix::net::UnixStream};

use crate::talker::Talker;

use super::device::{Device, ServiceRequest};

pub struct DataToSocketDevice {
    buffer: Vec<u8>,
    socket: UnixStream,
}

impl DataToSocketDevice {
    pub fn new(path: &str) -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
            socket: UnixStream::connect(path).expect("failed to connect to socket"),
        }
    }

    fn process_byte(&mut self, byte: u8, _eoi: bool) {
        self.buffer.push(byte);
    }

    fn process_complete(&mut self) {
        self.socket.write_all(&self.buffer).expect("failed to write to socket");
        self.buffer.clear();
    }
}

impl Device for DataToSocketDevice {
    fn reset(&mut self) {
        // Do nothing. The device has no state.
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> ServiceRequest {
        self.process_byte(byte, eoi);
        ServiceRequest::NotRequired
    }

    fn process_complete(&mut self) {
        self.process_complete();
    }

    fn talk(&mut self, _talker: Talker) {
        // Do nothing. The device cannot talk.
    }
}
