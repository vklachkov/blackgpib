mod identity;
mod request;
mod response;

use memmap2::MmapMut;

use crate::{debug, error, gpib};

use super::device::{Device, ServiceRequest};

use self::{
    identity::DiskIdentity,
    request::{Request, RequestCode},
    response::{DiskStatus, Response},
};

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
    Identity {
        full: bool,
    },
    Initialize {
        _sector: u32,
    },
    Read {
        sector: u32,
    },
    Write {
        sector: u32,
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
    identity: [u8; 56],
    image: MmapMut,
    state: State,
}

impl Disk {
    pub fn new(name: String, image: MmapMut) -> Self {
        let sector_remainder = image.len() % SECTOR_SIZE;
        if sector_remainder != 0 {
            panic!("Disk image must be multiple of {SECTOR_SIZE}");
        }

        let sector_count = (image.len() / SECTOR_SIZE) as u16;
        if sector_count < 6 {
            panic!("Disk image must be at least 6 sectors in size");
        }

        let identity = Self::identity(&name, sector_count).into_bytes();

        Self {
            identity,
            image,
            state: State::Idle,
        }
    }

    fn identity(name: &str, sector_count: u16) -> DiskIdentity {
        assert!(name.is_ascii(), "Device name must be ASCII");

        let mut device_name = [b' '; 32];
        let name_len = name.len().min(device_name.len());
        device_name[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);

        DiskIdentity {
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
            interleave_factor: 0,
            second_side_count: 0,
            num_cylinders: 0,
        }
    }

    fn reset(&mut self) {
        self.state = State::Idle;
    }

    fn process_bytes(&mut self, buffer: &[u8]) -> ServiceRequest {
        match self.state {
            State::Idle => {
                return self.process_new_request(buffer);
            }
            State::Initialize { .. } | State::Identity { .. } | State::Read { .. } => {
                panic!("unexpected data received");
            }
            State::Write { sector, state } => {
                if state != WriteDataState::NotAccepted {
                    panic!("unexpected data received")
                }

                return self.process_write_request(sector, buffer);
            }
            State::Format => {
                panic!("unexpected data received");
            }
            State::InvalidRequest => {
                panic!("unexpected data received");
            }
        }
    }

    fn process_new_request(&mut self, buffer: &[u8]) -> ServiceRequest {
        match Request::try_from(buffer) {
            Ok(req) => {
                debug!("Received {req:?}");
                self.process_request(req)
            }
            Err(err) => {
                error!("Failed to parse request {buffer:02x?}: {err}");

                self.state = State::InvalidRequest;

                ServiceRequest::NotRequired
            }
        }
    }

    fn process_request(&mut self, req: Request) -> ServiceRequest {
        match req.code {
            RequestCode::Initialize => {
                self.state = State::Initialize { _sector: req.sector };

                ServiceRequest::NotRequired
            }
            RequestCode::GetStatus => {
                self.state = State::Identity {
                    // Sometimes Compass may request 54 bytes of identifier,
                    // and in this case it is necessary to reply with exactly 52 bytes.
                    full: req.data_size == 56,
                };

                ServiceRequest::NotRequired
            }
            RequestCode::Read => {
                self.state = State::Read { sector: req.sector };

                ServiceRequest::Required
            }
            RequestCode::Write => {
                self.state = State::Write {
                    sector: req.sector,
                    state: WriteDataState::NotAccepted,
                };

                // SRQ required only after receiving a sector bytes for writing.
                ServiceRequest::NotRequired
            }
            RequestCode::Format => {
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

    fn process_write_request(&mut self, sector: u32, buffer: &[u8]) -> ServiceRequest {
        let offset = sector as usize * SECTOR_SIZE;

        let is_u32_max = sector == u32::MAX;
        let in_bounds = offset < self.image.len();

        let state = if is_u32_max {
            // Do nothing.
            // I do not know exactly why the laptop sends a write request to sector
            // 0xFFFFFFFF, but I think it is used to check the read data.
            WriteDataState::Checked
        } else if in_bounds {
            self.image[offset..offset + SECTOR_SIZE].copy_from_slice(buffer);
            WriteDataState::Saved
        } else {
            WriteDataState::OutOfBounds
        };

        self.state = State::Write { sector, state };

        return ServiceRequest::Required;
    }

    fn talk(&mut self, mut talker: gpib::Talker) {
        let response = self.response();
        talker.send_bytes(response.as_slice());

        // Reset to default state after answer because disk is stateless :)
        self.reset();
    }

    #[inline]
    fn response<'d>(&'d mut self) -> Response<'d> {
        match self.state {
            State::Idle => {
                // TODO: What should we return?
                panic!("Disk can't talk in idle state");
            }
            State::Initialize { .. } => {
                // TODO: What should we return?
                Response::ok(None)
            }
            State::Identity { full } => {
                let size = if full { 56 } else { 52 };
                Response::Raw(&self.identity[..size])
            }
            State::Read { sector } => {
                if &self.image[0..8] == &[0xe5; 8] {
                    debug!("Unformatted disk read detected");
                    return Response::from_status(DiskStatus::NotFormatted, 0x00);
                }

                let offset = sector as usize * SECTOR_SIZE;
                if offset >= self.image.len() {
                    // FIXME: What is correct response for this situation?
                    Response::ok(None)
                } else {
                    Response::Raw(&self.image[offset..offset + SECTOR_SIZE])
                }
            }
            State::Write { state, sector } => {
                if state == WriteDataState::NotAccepted {
                    // TODO: What should we return?
                    panic!("Disk can't talk while waiting for data");
                }

                if state == WriteDataState::OutOfBounds {
                    // TODO: What is correct response for this situation?
                    Response::ok(Some(sector))
                } else {
                    Response::ok(Some(sector))
                }
            }
            State::Format => Response::ok(None),
            State::InvalidRequest => Response::from_status(DiskStatus::UnsupportedCommand, 0x00),
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
