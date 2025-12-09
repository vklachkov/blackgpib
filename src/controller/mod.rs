mod disk_request;

use crate::{gpib_command::GPIBCommand, listener::Listener, talker::Talker};

use disk_request as request;

pub struct DeviceController {
    address: u8,
}

impl DeviceController {
    pub fn new(address: u8) -> Self {
        Self { address }
    }

    pub fn start(self) {
        let mut buffer = Vec::with_capacity(512);

        let mut talker = Talker::new();

        talker.send_command(GPIBCommand::DCL);
        talker.send_command(GPIBCommand::MLA(self.address));
        talker.send_bytes(
            &request::Request {
                code: request::RequestCode::GetStatus,
                connection: 0,
                sector: 0,
                data_size: 56,
                mode: 0,
            }
            .into_bytes(),
            true,
        );
        talker.send_command(GPIBCommand::UNL);
        talker.send_command(GPIBCommand::MTA(self.address));

        drop(talker);

        let mut listener = Listener::new(false);
        loop {
            let byte = listener.handshake_byte();
            buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        drop(listener);

        let mut talker = Talker::new();
        talker.send_command(GPIBCommand::UNT);

        println!("Buffer: {:x?} ({} bytes)", buffer, buffer.len());
    }
}
