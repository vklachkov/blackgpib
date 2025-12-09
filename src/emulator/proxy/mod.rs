use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use crate::talker::Talker;

use super::device::{Device, ServiceRequest};

pub struct DataToSocketDevice {
    buffer: Vec<u8>,
    socket: UdpSocket,
    port: u16,
}

impl DataToSocketDevice {
    pub fn new(port: u16) -> Self {
        let addr = (IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let socket = UdpSocket::bind(addr).expect("failed to bind to socket");
        socket.set_broadcast(true).expect("failed to set_broadcast for socket");

        Self {
            buffer: Vec::with_capacity(1024),
            socket,
            port,
        }
    }

    fn process_byte(&mut self, byte: u8, _eoi: bool) {
        self.buffer.push(byte);
    }

    fn process_complete(&mut self) -> ServiceRequest {
        self.socket
            .send_to(&self.buffer, (IpAddr::V4(Ipv4Addr::BROADCAST), self.port))
            .expect("failed to write to socket");

        self.buffer.clear();

        ServiceRequest::NotRequired
    }
}

impl Device for DataToSocketDevice {
    fn reset(&mut self) {
        // Do nothing. The device has no state.
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) {
        self.process_byte(byte, eoi);
    }

    fn unlisten(&mut self) -> ServiceRequest {
        self.process_complete()
    }

    fn talk(&mut self, _talker: Talker) {
        // Do nothing. The device cannot talk.
    }
}
