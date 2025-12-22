use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use crate::talker::Talker;

use super::device::{Device, ServiceRequest};

pub struct DataToSocketDevice {
    socket: UdpSocket,
    port: u16,
}

impl DataToSocketDevice {
    pub fn new(port: u16) -> Self {
        let addr = (IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let socket = UdpSocket::bind(addr).expect("failed to bind to socket");
        socket.set_broadcast(true).expect("failed to set_broadcast for socket");

        Self { socket, port }
    }

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest {
        self.socket
            .send_to(buffer, (IpAddr::V4(Ipv4Addr::BROADCAST), self.port))
            .expect("failed to write to socket");

        ServiceRequest::NotRequired
    }
}

impl Device for DataToSocketDevice {
    fn reset(&mut self) {
        // Do nothing. The device has no state.
    }

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest {
        self.process_bytes(buffer)
    }

    fn talk(&mut self, _talker: Talker) {
        // Do nothing. The device cannot talk.
    }
}
