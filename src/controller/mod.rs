mod disk_request;

use std::fs;

use crate::{gpib_command::GPIBCommand, listener::Listener, talker::Talker, utils::busy_wait};

pub struct DeviceController {
    address: u8,
}

impl DeviceController {
    pub fn new(address: u8) -> Self {
        Self { address }
    }

    pub fn start(self) {
        let mut buffer: Vec<u8> = Vec::with_capacity(512);

        let image = fs::read("MS-DOS 2.00A.img").unwrap();

        for i in 0..(image.len() / 512) {
            self.write_sector(&image[i * 512..i * 512 + 512], i as _, &mut buffer);
        }
    }

    fn write_sector(&self, sector_data: &[u8], sector_number: u32, buffer: &mut Vec<u8>) {
        let mut talker = Talker::new();

        talker.send_command(GPIBCommand::DCL);

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::Write,
                connection: 0,
                sector: sector_number,
                data_size: 512,
                mode: 0,
            }
            .into_bytes(),
        );

        self.send_bytes_with_handshake(&mut talker, sector_data);

        // println!("Wait srq...");
        talker.wait_srq();
        // println!("Got it!");

        talker.send_command(GPIBCommand::SPE);

        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        self.read_serial_poll();

        talker = Talker::new();

        talker.send_command(GPIBCommand::SPD);
        talker.send_command(GPIBCommand::UNT);

        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        self.read_and_println(buffer);
        assert_eq!(buffer.len(), 7);
        assert_eq!(buffer[0], 0x00);

        // println!("Read after write...");

        let mut talker = Talker::new();

        talker.send_command(GPIBCommand::DCL);

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::Read,
                connection: 0,
                sector: sector_number,
                data_size: 512,
                mode: 0,
            }
            .into_bytes(),
        );

        // println!("Wait srq...");
        talker.wait_srq();
        // println!("Got it!");

        talker.send_command(GPIBCommand::SPE);

        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        self.read_serial_poll();

        talker = Talker::new();

        talker.send_command(GPIBCommand::SPD);
        talker.send_command(GPIBCommand::UNT);

        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        self.read_and_println(buffer);
        if buffer.len() != 512 {
            println!("Bad response: {:02x?} ({} bytes)", buffer, buffer.len());
        }
        else if &buffer[0..512] != sector_data {
            println!("Data mismatch");
        }
        buffer.clear();
    }

    #[inline(always)]
    fn send_bytes_with_handshake(&self, talker: &mut Talker, bytes: &[u8]) {
        talker.send_command(GPIBCommand::MLA(self.address));
        talker.send_bytes(bytes, true);
        talker.send_command(GPIBCommand::UNL);
    }

    #[inline(always)]
    fn read_serial_poll(&self) {
        let mut listener = Listener::new(false);
        listener.handshake_byte();
        // println!("Serial poll byte {:02x}", byte.value);
    }

    #[inline(always)]
    fn read_and_println(&self, buffer: &mut Vec<u8>) {
        buffer.clear();

        let mut listener = Listener::new(false);
        loop {
            let byte = listener.handshake_byte();
            buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        // println!("Buffer: {:02x?} ({} bytes)", buffer, buffer.len());
    }
}
