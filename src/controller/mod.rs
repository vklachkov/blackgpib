use std::{io, time::Duration};

use crate::{
    debug,
    disk_protocol::{Request, RequestCode, Status, StatusResponse, StatusResponseErrno},
    gpib::{Command, Listener, Talker},
    gpio::Gpio,
    info,
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
        info!("Reset device");
        Talker::new(&mut gpio).send_command(Command::DCL);

        info!("Wait 15 seconds...");
        busy_wait(Duration::from_secs(15));

        Self {
            gpio,
            address,
            buffer: Vec::with_capacity(SECTOR_SIZE),
        }
    }

    pub fn read_status(&mut self) -> io::Result<Status> {
        self.send_get_status_cmd();

        self.response_handshake();
        let status = Status::try_from_bytes(&self.buffer[0..52]).unwrap();
        Ok(status)
    }

    pub fn format_disk(&mut self, validate: bool) -> io::Result<()> {
        self.low_level_format()?;
        info!("Low-level formatting completed");

        if !validate {
            info!("Disk validation skipped");
            return Ok(());
        }

        let status = self.read_status()?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            let sector = self.read_sector(i as u32)?;

            let has_empty_marker = sector.iter().take(8).all(|b| *b == 0xff);
            let has_uninit_bytes = sector.iter().skip(8).all(|b| *b == 0xe5);

            if !has_empty_marker || !has_uninit_bytes {
                return Err(io::Error::other(format!("sector {}/{} validation failed", i + 1, sector_count)));
            }

            info!("Sector {}/{} verified", i + 1, sector_count);
        }

        Ok(())
    }

    fn low_level_format(&mut self) -> io::Result<()> {
        self.send_format_cmd();

        self.serial_poll_handshake()?;

        self.response_handshake();
        self.check_status()?;

        Ok(())
    }

    pub fn read_disk_to_writer(&mut self, mut w: impl io::Write) -> io::Result<()> {
        let status = self.read_status()?;
        let sector_count = status.sector_count;

        for i in 0..sector_count {
            info!("Sector {}/{} read", i + 1, sector_count);

            let sector = self.read_sector(i as u32)?;
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

            info!("Write sector {}/{}", i + 1, sector_count);

            self.write_sector(i as u32, &data)?;

            let sector = self.read_sector(i as u32)?;
            if sector != data {
                return Err(io::Error::other("read write data mismatch"));
            }
        }

        Ok(())
    }

    fn write_sector(&mut self, sector: u32, data: &[u8]) -> io::Result<()> {
        self.send_write_cmd(sector, data);

        self.serial_poll_handshake()?;

        self.response_handshake();
        self.check_status()?;

        Ok(())
    }

    fn read_sector(&mut self, sector: u32) -> io::Result<&[u8]> {
        self.send_read_cmd(sector);

        self.serial_poll_handshake()?;

        self.response_handshake();
        self.check_status()?;

        return Ok(&self.buffer);
    }

    fn send_get_status_cmd(&mut self) {
        let mut talker = Talker::new(&mut self.gpio);

        debug!("Send GetStatus command");
        Self::send_request_with_handshake(
            &mut talker,
            self.address,
            Request::new(RequestCode::GET_STATUS, None, 54), // 54 like real Compass
        );
    }

    fn send_format_cmd(&mut self) {
        let mut talker = Talker::new(&mut self.gpio);

        debug!("Send Format command");
        Self::send_request_with_handshake(
            &mut talker,
            self.address,
            Request::new(RequestCode::FORMAT, None, 1), // 1 like real Compass
        );
    }

    fn send_write_cmd(&mut self, sector: u32, data: &[u8]) {
        assert!(data.len() == SECTOR_SIZE);

        let mut talker = Talker::new(&mut self.gpio);

        debug!("Send Write({sector}) command");
        Self::send_request_with_handshake(
            &mut talker,
            self.address,
            Request::new(RequestCode::WRITE, Some(sector), SECTOR_SIZE as _),
        );

        Self::send_bytes_with_handshake(&mut talker, self.address, data);
    }

    fn send_read_cmd(&mut self, sector: u32) {
        let mut talker = Talker::new(&mut self.gpio);

        debug!("Send Read({sector}) command");
        Self::send_request_with_handshake(
            &mut talker,
            self.address,
            Request::new(RequestCode::READ, Some(sector), SECTOR_SIZE as _),
        );
    }

    fn serial_poll_handshake(&mut self) -> io::Result<()> {
        debug!("Waiting for device to become ready...");

        let mut talker = Talker::new(&mut self.gpio);

        talker.wait_srq();

        talker.send_command(Command::SPE);

        talker.send_command(Command::MTA(self.address));

        let listener = Listener::new(&mut self.gpio);
        if (*listener.start_data_handshake()).value != 0x4F {
            return Err(io::Error::other("found another device on the bus"));
        }

        talker = Talker::new(&mut self.gpio);

        talker.send_command(Command::SPD);
        talker.send_command(Command::UNT);

        debug!("Device ready");

        Ok(())
    }

    fn response_handshake(&mut self) {
        let talker = Talker::new(&mut self.gpio);
        talker.send_command(Command::MTA(self.address));

        self.buffer.clear();

        let listener = Listener::new(&mut self.gpio);
        loop {
            let byte = *listener.start_data_handshake();
            self.buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        let talker = Talker::new(&mut self.gpio);
        talker.send_command(Command::UNT);
    }

    fn check_status(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty response from disk"));
        }

        if self.buffer.len() == SECTOR_SIZE {
            return Ok(());
        }

        let Ok(status_response) = StatusResponse::try_from_bytes(&self.buffer) else {
            return Err(io::Error::other(format!(
                "unexpected response length from disk: got {} bytes, expected {SECTOR_SIZE} or 7 byte",
                self.buffer.len(),
            )));
        };

        if status_response.status != StatusResponseErrno::OK {
            return Err(io::Error::other(format!("disk returns an error {:?}", status_response.status)));
        }

        Ok(())
    }

    fn send_request_with_handshake(talker: &mut Talker, address: u8, request: Request) {
        Self::send_bytes_with_handshake(talker, address, &request.into_bytes());
    }

    fn send_bytes_with_handshake(talker: &mut Talker, address: u8, bytes: &[u8]) {
        talker.send_command(Command::MLA(address));
        talker.send_bytes(bytes);
        talker.send_command(Command::UNL);
    }
}
