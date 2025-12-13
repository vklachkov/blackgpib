mod disk_request;

use std::io;

use crate::{gpib_command::GPIBCommand, listener::Listener, talker::Talker};

const SECTOR_SIZE: usize = 512;

pub struct DeviceController {
    address: u8,
    buffer: Vec<u8>,
}

impl DeviceController {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            buffer: Vec::with_capacity(512),
        }
    }

    pub fn reset_device(&self) {
        let mut talker = Talker::new();
        talker.send_command(GPIBCommand::DCL);
    }

    pub fn format_disk(&mut self) {
        self.low_level_format();
        println!("Low level format complete!");

        for i in 0..720 {
            let sector = self.read_sector(i as u32);
            assert_eq!(&sector[..8], &[0xff; 8], "bad sector after format");
            assert_eq!(&sector[8..], &[0xe5; 504], "bad sector after format");
            println!("Sector {}/720 verified", i+1);
        }
    }

    fn low_level_format(&mut self) {
        self.send_format_cmd();

        self.serial_poll_handshake();

        self.response_handshake();
        assert_eq!(self.buffer[0], 0x00, "format failed");
    }

    pub fn read_disk_to_image(&mut self) {
        panic!("TODO");
    }

    pub fn write_image_to_disk(&mut self, image: &[u8]) -> io::Result<()> {
        if image.len() % SECTOR_SIZE != 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let total_sectors = image.len() / SECTOR_SIZE;
        for i in 0..total_sectors {
            let offset = i * SECTOR_SIZE;
            let data = &image[offset..offset + SECTOR_SIZE];

            self.write_sector(i as u32, data);

            let sector = self.read_sector(i as u32);
            assert_eq!(sector, data, "read write data mismatch");
        }

        Ok(())
    }

    fn write_sector(&mut self, sector: u32, data: &[u8]) {
        self.send_write_cmd(sector, data);

        self.serial_poll_handshake();

        self.response_handshake();
        assert!(self.buffer[0] == 0x00, "write failed {:#04x}", self.buffer[0]);
    }

    fn read_sector<'a>(&'a mut self, sector: u32) -> &'a [u8] {
        self.send_read_cmd(sector);

        self.serial_poll_handshake();

        self.response_handshake();

        if self.buffer.len() == 7 {
            assert!(self.buffer[0] == 0x00, "read failed with status {:#04x}", self.buffer[0]);
        }

        assert_eq!(self.buffer.len(), SECTOR_SIZE, "weird `read` response length from device");
        return &self.buffer;
    }

    fn send_format_cmd(&self) {
        let mut talker = Talker::new();

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::Format,
                connection: 0,
                sector: 0,
                data_size: 1,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn send_write_cmd(&self, sector: u32, data: &[u8]) {
        assert!(data.len() == SECTOR_SIZE);

        let mut talker = Talker::new();

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::Write,
                connection: 0,
                sector,
                data_size: SECTOR_SIZE as _,
                mode: 0,
            }
            .into_bytes(),
        );

        self.send_bytes_with_handshake(&mut talker, data);
    }

    fn send_read_cmd(&self, sector: u32) {
        let mut talker = Talker::new();

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::Read,
                connection: 0,
                sector,
                data_size: SECTOR_SIZE as _,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn serial_poll_handshake(&self) {
        let mut talker = Talker::new();

        talker.wait_srq();

        talker.send_command(GPIBCommand::SPE);

        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        let mut listener = Listener::new(false);
        assert_eq!(listener.handshake_byte().value, 0x4F);
        drop(listener);

        talker = Talker::new();

        talker.send_command(GPIBCommand::SPD);
        talker.send_command(GPIBCommand::UNT);
    }

    fn response_handshake(&mut self) {
        let mut talker = Talker::new();
        talker.send_command(GPIBCommand::MTA(self.address));
        drop(talker);

        self.buffer.clear();

        let mut listener = Listener::new(false);
        loop {
            let byte = listener.handshake_byte();
            self.buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        drop(listener);

        let mut talker = Talker::new();
        talker.send_command(GPIBCommand::UNT);
        drop(talker);        
    }

    #[inline(always)]
    fn send_bytes_with_handshake(&self, talker: &mut Talker, bytes: &[u8]) {
        talker.send_command(GPIBCommand::MLA(self.address));
        talker.send_bytes(bytes, true);
        talker.send_command(GPIBCommand::UNL);
    }
}
