use std::io;

use memmap2::MmapMut;

use crate::{
    debug,
    disk_protocol::{Request, RequestCode, Response, Status, StatusResponseErrno},
    error, gpib,
};

use super::device::{Device, ServiceRequest};

/// Actual sector size. This size is used by both the hard disk and floppy drive.
///
/// The laptop does not support a different sector size for disks connected
/// via GPIB. The sector size is hardcoded everywhere, from bootloader to firmwares.
const SECTOR_SIZE: usize = 512;

/// The number of bytes in a sector that can be used.
///
/// Does not include the first 4 bytes or the last 4 bytes of the sector.
/// For more details about the CCOS file system structure, see the repository
/// by @BOOtak: https://github.com/BOOtak/CCOS-disk-utils/.
const LOGICAL_SECTOR_SIZE: usize = SECTOR_SIZE - 8;

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    Idle,
    Status,
    Initialize,
    Read {
        sector: u32,
    },
    Write {
        sector: u32,
        mode: u8,
        state: WriteDataState,
    },
    Format,
    InvalidRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteDataState {
    NotAccepted,
    Saved,
    Checked,
    OutOfBounds,
}

pub struct Disk {
    status: [u8; 52],
    image: MmapMut,
    state: State,
}

impl Disk {
    pub fn new(name: String, image: MmapMut) -> io::Result<Self> {
        let sector_remainder = image.len() % SECTOR_SIZE;
        if sector_remainder != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Disk image must be multiple of {SECTOR_SIZE}"),
            ));
        }

        let sector_count = (image.len() / SECTOR_SIZE) as u16;
        if sector_count < 6 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Disk image must be at least 6 sectors in size"));
        }

        let status = Self::status(&name, sector_count).into_bytes();

        Ok(Self {
            status,
            image,
            state: State::Idle,
        })
    }

    fn status(name: &str, sector_count: u16) -> Status {
        assert!(name.is_ascii(), "Device name must be ASCII");

        let mut device_name = [b' '; 32];
        let name_len = name.len().min(device_name.len());
        device_name[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);

        Status {
            sector_size: SECTOR_SIZE as _,
            logical_sector_size: LOGICAL_SECTOR_SIZE as _,
            sector_count,
            drive_status: 1,
            bitmap_block_id: 0x120,
            superblock_id: 0x121,
            min_dir_pages: 1,
            flush: 0,
            device_name,
            bytes_per_sector: SECTOR_SIZE as _,
            sectors_per_track: 0,
            tracks_per_cylinder: 0,
        }
    }

    fn reset(&mut self) {
        self.state = State::Idle;
    }

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest {
        match self.state {
            State::Write {
                sector,
                mode,
                state: WriteDataState::NotAccepted,
            } => {
                return self.process_write_request(sector, mode, buffer);
            }
            _ => {
                return self.process_new_request(buffer);
            }
        }
    }

    fn process_new_request(&mut self, buffer: &[u8]) -> ServiceRequest {
        match Request::try_from_bytes(buffer) {
            Ok(req) => {
                debug!("Received {req:?}");
                self.process_request(req)
            }
            Err(buffer) => {
                error!("Invalid request: {buffer:02x?}");

                self.state = State::InvalidRequest;

                ServiceRequest::NotRequired
            }
        }
    }

    fn process_request(&mut self, req: Request) -> ServiceRequest {
        match req.code {
            RequestCode::INITIALIZE => {
                self.state = State::Initialize;

                ServiceRequest::NotRequired
            }
            RequestCode::GET_STATUS => {
                self.state = State::Status;

                ServiceRequest::NotRequired
            }
            RequestCode::READ => {
                self.state = State::Read { sector: req.sector };

                ServiceRequest::Required
            }
            RequestCode::WRITE => {
                self.state = State::Write {
                    sector: req.sector,
                    mode: req.mode,
                    state: WriteDataState::NotAccepted,
                };

                // SRQ required only after receiving a sector bytes for writing.
                ServiceRequest::NotRequired
            }
            RequestCode::FORMAT => {
                self.format_image();
                self.state = State::Format;
                ServiceRequest::Required
            }
            _ => {
                self.state = State::InvalidRequest;
                ServiceRequest::NotRequired
            }
        }
    }

    fn format_image(&mut self) {
        let blocks = self.image.len() / SECTOR_SIZE;
        for i in 0..blocks {
            let offset = i * SECTOR_SIZE;
            let sector = &mut self.image[offset..offset + SECTOR_SIZE];
            sector[0..8].fill(0xff);
            sector[8..SECTOR_SIZE].fill(0xe5);
        }
    }

    fn process_write_request(&mut self, sector: u32, mode: u8, buffer: &[u8]) -> ServiceRequest {
        let offset = sector as usize * SECTOR_SIZE;

        let is_prev_data_check = mode == 1;
        let in_bounds = offset < self.image.len();

        let state = if is_prev_data_check {
            // Do nothing. The laptop can send a Write(0xFFFFFFFF) or Write(0xFFFF) request with mode=1
            // with the data you sent before. It looks like this is a validation request.
            //
            // If there are less than 512 bytes sent, for example the response for GetStatus,
            // then the rest of the bytes will be filled with some garbage from memory.
            //
            // Like a real floppy drive, always response OK.
            WriteDataState::Checked
        } else if in_bounds {
            self.image[offset..offset + SECTOR_SIZE].copy_from_slice(buffer);
            WriteDataState::Saved
        } else {
            WriteDataState::OutOfBounds
        };

        self.state = State::Write { sector, mode, state };

        return ServiceRequest::Required;
    }

    fn talk(&mut self, mut talker: gpib::Talker) {
        match self.response() {
            Response::Raw(bytes) => talker.send_bytes(bytes),
            Response::Status(s) => talker.send_bytes(&s.into_bytes()),
        }

        // Reset to default state after answer because disk is stateless :)
        self.reset();
    }

    #[inline]
    fn response<'d>(&'d mut self) -> Response<'d> {
        match self.state {
            State::Idle => {
                Response::Raw(&[]) //
            }
            State::Initialize => {
                Response::ok(None) //
            }
            State::Status => {
                // Sometimes Compass may request 54 bytes of identifier,
                // but emulator must reply with exactly 52 bytes.
                Response::Raw(&self.status)
            }
            State::Read { sector } => {
                if self.image[0..8] == [0xe5; 8] {
                    debug!("Unformatted disk read detected");
                    return Response::from_status(StatusResponseErrno::NOT_FORMATTED, sector as _);
                }

                let offset = sector as usize * SECTOR_SIZE;
                if offset >= self.image.len() {
                    Response::from_status(StatusResponseErrno::OUT_OF_BOUNDS, sector as _) //
                } else {
                    Response::Raw(&self.image[offset..offset + SECTOR_SIZE]) //
                }
            }
            State::Write { state, sector, .. } => match state {
                WriteDataState::NotAccepted => {
                    Response::Raw(&[]) //
                }
                WriteDataState::Saved => {
                    Response::ok(Some(sector)) //
                }
                WriteDataState::Checked => {
                    Response::ok(Some(0xFFFF)) //
                }
                WriteDataState::OutOfBounds => {
                    Response::from_status(StatusResponseErrno::OUT_OF_BOUNDS, sector as _) //
                }
            },
            State::Format => {
                Response::ok(None) //
            }
            State::InvalidRequest => {
                Response::from_status(StatusResponseErrno::UNSUPPORTED, 0x00) //
            }
        }
    }
}

impl Device for Disk {
    fn reset(&mut self) {
        self.reset();
    }

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest {
        self.process_bytes(buffer)
    }

    fn talk(&mut self, talker: gpib::Talker) {
        self.talk(talker)
    }
}
