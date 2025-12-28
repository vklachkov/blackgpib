use std::{io, time::Duration};

use crate::{
    disk_protocol::{DiskIdentity, Request, RequestCode},
    gpib::{Command, Listener, Talker},
    gpio::Gpio,
    time_utils::busy_wait,
};

const SECTOR_SIZE: usize = 512;

pub struct DeviceController {
    gpio: Gpio,
    address: u8,
    buffer: Vec<u8>,
}

impl DeviceController {
    pub fn new_with_reset(mut gpio: Gpio, address: u8) -> Self {
        Talker::new(&mut gpio).send_command(Command::DCL);

        crate::trace!("Wait 15 seconds...");
        busy_wait(Duration::from_secs(15));

        Self {
            gpio,
            address,
            buffer: Vec::with_capacity(512),
        }
    }

    pub fn read_status(&mut self) -> io::Result<DiskIdentity> {
        self.send_get_status_cmd();

        self.response_handshake();
        let status = DiskIdentity::try_from_bytes(&self.buffer[0..52]).unwrap();
        Ok(status)
    }

    pub fn format_disk(&mut self, validate: bool) -> io::Result<()> {
        self.low_level_format();

        if !validate {
            return Ok(());
        }

        let status = self.read_status()?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let sector = self.read_sector(i as u32);
            assert_eq!(&sector[..8], &[0xff; 8], "bad sector after format");
            assert_eq!(&sector[8..], &[0xe5; 504], "bad sector after format");
            println!("Sector {}/{} verified", i + 1, sector_count);
        }

        Ok(())
    }

    fn low_level_format(&mut self) {
        self.send_format_cmd();

        self.serial_poll_handshake();

        self.response_handshake();
        assert_eq!(self.buffer[0], 0x00, "format failed");
    }

    pub fn read_disk_to_writer(&mut self, mut w: impl io::Write) -> io::Result<()> {
        let status = self.read_status()?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let sector = self.read_sector(i as u32);
            w.write_all(sector)?;
        }

        Ok(())
    }

    pub fn write_image_to_disk(&mut self, mut r: impl io::Read) -> io::Result<()> {
        let status = self.read_status()?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let mut data = [0u8; SECTOR_SIZE];
            r.read_exact(&mut data)?;
            self.write_sector(i as u32, &data);

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

    fn send_get_status_cmd(&mut self) {
        let mut talker = Talker::new(&mut self.gpio);

        Self::send_bytes_with_handshake(
            &mut talker,
            self.address,
            &Request {
                code: RequestCode::GetStatus,
                connection: 0,
                sector: 0,
                data_size: 52,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn send_format_cmd(&mut self) {
        let mut talker = Talker::new(&mut self.gpio);

        Self::send_bytes_with_handshake(
            &mut talker,
            self.address,
            &Request {
                code: RequestCode::Format,
                connection: 0,
                sector: 0,
                data_size: 1,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn send_write_cmd(&mut self, sector: u32, data: &[u8]) {
        assert!(data.len() == SECTOR_SIZE);

        let mut talker = Talker::new(&mut self.gpio);

        Self::send_bytes_with_handshake(
            &mut talker,
            self.address,
            &Request {
                code: RequestCode::Write,
                connection: 0,
                sector,
                data_size: SECTOR_SIZE as _,
                mode: 0,
            }
            .into_bytes(),
        );

        Self::send_bytes_with_handshake(&mut talker, self.address, data);
    }

    fn send_read_cmd(&mut self, sector: u32) {
        let mut talker = Talker::new(&mut self.gpio);

        Self::send_bytes_with_handshake(
            &mut talker,
            self.address,
            &Request {
                code: RequestCode::Read,
                connection: 0,
                sector,
                data_size: SECTOR_SIZE as _,
                mode: 0,
            }
            .into_bytes(),
        );
    }

    fn serial_poll_handshake(&mut self) {
        let mut talker = Talker::new(&mut self.gpio);

        talker.wait_srq();

        talker.send_command(Command::SPE);

        talker.send_command(Command::MTA(self.address));
        drop(talker);

        let listener = Listener::new(&mut self.gpio);
        assert_eq!((*listener.start_data_handshake()).value, 0x4F);
        drop(listener);

        talker = Talker::new(&mut self.gpio);

        talker.send_command(Command::SPD);
        talker.send_command(Command::UNT);
    }

    fn response_handshake(&mut self) {
        let talker = Talker::new(&mut self.gpio);
        talker.send_command(Command::MTA(self.address));
        drop(talker);

        self.buffer.clear();

        let listener = Listener::new(&mut self.gpio);
        loop {
            let byte = *listener.start_data_handshake();
            self.buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        drop(listener);

        let talker = Talker::new(&mut self.gpio);
        talker.send_command(Command::UNT);
        drop(talker);
    }

    fn send_bytes_with_handshake(talker: &mut Talker, address: u8, bytes: &[u8]) {
        talker.send_command(Command::MLA(address));
        talker.send_bytes(bytes);
        talker.send_command(Command::UNL);
    }
}
