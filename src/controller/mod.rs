mod disk_identity;
mod disk_request;

use std::io;

use crate::{
    gpib::{Command, Listener, Talker},
    gpio::Gpio,
};

const SECTOR_SIZE: usize = 512;

pub struct DeviceController {
    address: u8,
    buffer: Vec<u8>,
}

impl DeviceController {
    pub fn new_with_reset(address: u8, gpio: &mut Gpio) -> Self {
        Talker::new(gpio).send_command(Command::DCL);

        Self {
            address,
            buffer: Vec::with_capacity(512),
        }
    }

    pub fn read_status(&mut self, gpio: &mut Gpio) -> io::Result<disk_identity::DiskIdentity> {
        self.send_get_status_cmd(gpio);

        self.response_handshake(gpio);
        let status = disk_identity::DiskIdentity::try_from_bytes(&self.buffer[0..52]).unwrap();
        Ok(status)
    }

    pub fn format_disk(&mut self, validate: bool, gpio: &mut Gpio) -> io::Result<()> {
        self.low_level_format(gpio);

        if !validate {
            return Ok(());
        }

        let status = self.read_status(gpio)?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let sector = self.read_sector(i as u32, gpio);
            assert_eq!(&sector[..8], &[0xff; 8], "bad sector after format");
            assert_eq!(&sector[8..], &[0xe5; 504], "bad sector after format");
            println!("Sector {}/{} verified", i + 1, sector_count);
        }

        Ok(())
    }

    fn low_level_format(&mut self, gpio: &mut Gpio) {
        self.send_format_cmd(gpio);

        self.serial_poll_handshake(gpio);

        self.response_handshake(gpio);
        assert_eq!(self.buffer[0], 0x00, "format failed");
    }

    pub fn read_disk_to_writer(&mut self, mut w: impl io::Write, gpio: &mut Gpio) -> io::Result<()> {
        let status = self.read_status(gpio)?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let sector = self.read_sector(i as u32, gpio);
            w.write_all(sector)?;
        }

        Ok(())
    }

    pub fn write_image_to_disk(&mut self, mut r: impl io::Read, gpio: &mut Gpio) -> io::Result<()> {
        let status = self.read_status(gpio)?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let mut data = [0u8; SECTOR_SIZE];
            r.read_exact(&mut data)?;
            self.write_sector(i as u32, &data, gpio);

            let sector = self.read_sector(i as u32, gpio);
            assert_eq!(sector, data, "read write data mismatch");
        }

        Ok(())
    }

    fn write_sector(&mut self, sector: u32, data: &[u8], gpio: &mut Gpio) {
        self.send_write_cmd(sector, data, gpio);

        self.serial_poll_handshake(gpio);

        self.response_handshake(gpio);
        assert!(self.buffer[0] == 0x00, "write failed {:#04x}", self.buffer[0]);
    }

    fn read_sector<'a>(&'a mut self, sector: u32, gpio: &mut Gpio) -> &'a [u8] {
        self.send_read_cmd(sector, gpio);

        self.serial_poll_handshake(gpio);

        self.response_handshake(gpio);

        if self.buffer.len() == 7 {
            assert!(self.buffer[0] == 0x00, "read failed with status {:#04x}", self.buffer[0]);
        }

        assert_eq!(self.buffer.len(), SECTOR_SIZE, "weird `read` response length from device");
        return &self.buffer;
    }

    fn send_get_status_cmd(&self, gpio: &mut Gpio) {
        let mut talker = Talker::new(gpio);

        self.send_bytes_with_handshake(
            &mut talker,
            &disk_request::Request {
                code: disk_request::RequestCode::GetStatus,
                connection: 0,
                sector: 0,
                data_size: 52,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn send_format_cmd(&self, gpio: &mut Gpio) {
        let mut talker = Talker::new(gpio);

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

    fn send_write_cmd(&self, sector: u32, data: &[u8], gpio: &mut Gpio) {
        assert!(data.len() == SECTOR_SIZE);

        let mut talker = Talker::new(gpio);

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

    fn send_read_cmd(&self, sector: u32, gpio: &mut Gpio) {
        let mut talker = Talker::new(gpio);

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

    fn serial_poll_handshake(&self, gpio: &mut Gpio) {
        let mut talker = Talker::new(gpio);

        talker.wait_srq();

        talker.send_command(Command::SPE);

        talker.send_command(Command::MTA(self.address));
        drop(talker);

        let listener = Listener::new(gpio);
        assert_eq!((*listener.start_data_handshake()).value, 0x4F);
        drop(listener);

        talker = Talker::new(gpio);

        talker.send_command(Command::SPD);
        talker.send_command(Command::UNT);
    }

    fn response_handshake(&mut self, gpio: &mut Gpio) {
        let talker = Talker::new(gpio);
        talker.send_command(Command::MTA(self.address));
        drop(talker);

        self.buffer.clear();

        let listener = Listener::new(gpio);
        loop {
            let byte = *listener.start_data_handshake();
            self.buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        drop(listener);

        let talker = Talker::new(gpio);
        talker.send_command(Command::UNT);
        drop(talker);
    }

    fn send_bytes_with_handshake(&self, talker: &mut Talker, bytes: &[u8]) {
        talker.send_command(Command::MLA(self.address));
        talker.send_bytes(bytes);
        talker.send_command(Command::UNL);
    }
}
