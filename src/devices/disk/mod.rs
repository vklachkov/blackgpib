mod identity;
mod request;

use crate::{debug, error, talker::Talker, trace};

use super::device::{Device, ServiceRequest};

use identity::DiskIdentity;
use request::{Request, RequestCode};

use memmap2::MmapMut;

/// Actual sector size. This size is used by both the hard disk and floppy drive.
///
/// The laptop does not support a different sector size for disks connected
/// via GPIB. The sector size is hardcoded in the laptop bootloader, so this
/// parameter is specified as a constant.
const SECTOR_SIZE: usize = 512;

/// The number of bytes in a sector that can be used.
///
/// Does not include the first 4 bytes or the last 4 bytes of the sector.
/// For more details about the CCOS file system structure, see the repository
/// by @BOOtak: https://github.com/BOOtak/CCOS-disk-utils/.
const LOGICAL_SECTOR_SIZE: usize = 504;

// These responses were obtained through reverse engineering.
// The exact purpose of the bytes is unknown.
const NO_DISK_RESPONSE: [u8; 7] = [0x6b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
const OPERATION_DONE_RESPONSE: [u8; 7] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteDataState {
    NotAccepted,
    Saved,
    Checked,
    OutOfBounds,
}

pub struct Disk {
    name: String,
    identity: [u8; 56],
    image: Option<MmapMut>,
    buffer: Vec<u8>,
    state: State,
}

impl Disk {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            identity: Self::identity(name, 0xFFFF, 0xFFFF, 0xFFFF).into_bytes(),
            image: None,
            buffer: Vec::with_capacity(SECTOR_SIZE),
            state: State::Idle,
        }
    }

    fn identity(name: &str, sector_count: u16, superblock_id: u16, bitmap_block_id: u16) -> DiskIdentity {
        assert!(name.is_ascii(), "Device name must be ASCII");

        let mut device_name = [b' '; 32];
        let name_len = name.len().min(device_name.len());
        device_name[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);

        DiskIdentity {
            sector_size: SECTOR_SIZE as _,
            logical_sector_size: LOGICAL_SECTOR_SIZE as _,
            sector_count,
            drive_ready: true,
            bitmap_block_id,
            superblock_id,
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

    pub fn use_image(&mut self, image: MmapMut, superblock_id: u16, bitmap_block_id: u16) {
        // Make the image size a multiple of the sector size.
        let sector_remainder = image.len() % SECTOR_SIZE;
        if sector_remainder != 0 {
            panic!("Image must be multiple of {SECTOR_SIZE}!");
        }

        let sector_count = (image.len() / SECTOR_SIZE) as u16;
        self.identity = Self::identity(&self.name, sector_count, superblock_id, bitmap_block_id).into_bytes();

        self.image = Some(image);
    }

    pub fn has_image(&self) -> bool {
        self.image.is_some()
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.state = State::Idle;
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> ServiceRequest {
        self.buffer.push(byte);

        if eoi {
            self.process_buffer()
        } else {
            // Data is being received, service request is not needed.
            ServiceRequest::NotRequired
        }
    }

    fn process_buffer(&mut self) -> ServiceRequest {
        match self.state {
            State::Idle => {
                return self.process_new_request();
            }
            State::Initialize { .. } | State::Identity { .. } | State::Read { .. } => {
                panic!("unexpected data received");
            }
            State::Write { sector, state } => {
                if state != WriteDataState::NotAccepted {
                    panic!("unexpected data received")
                }

                return self.process_write_request(sector);
            }
            State::Format => {
                panic!("unexpected data received");
            }
        }
    }

    fn process_new_request(&mut self) -> ServiceRequest {
        let raw = self.buffer.as_slice();

        trace!("Parse disk request {raw:02x?}...");

        let req = match Request::try_from(raw) {
            Ok(value) => value,
            Err(err) => {
                error!("Failed to parse request: {err}");
                return ServiceRequest::NotRequired;
            }
        };

        debug!("Received disk request {req:?}");

        self.state = match req.code {
            RequestCode::Initialize => State::Initialize {
                _sector: req.sector,
                // TODO: What means data_size=0xFFFF?
            },
            RequestCode::GetStatus => State::Identity {
                // Sometimes Compass may request 54 bytes of identifier,
                // and in this case it is necessary to reply with exactly 52 bytes.
                full: req.data_size == 56,
            },
            RequestCode::Read => State::Read { sector: req.sector },
            RequestCode::Write => State::Write {
                sector: req.sector,
                state: WriteDataState::NotAccepted,
            },
            RequestCode::Format => {
                self.format_image();
                State::Format
            }
            _ => panic!("Unexpected request {req:?}"),
        };
        self.buffer.clear();

        return if req.code == RequestCode::Read || req.code == RequestCode::Format {
            ServiceRequest::Required
        } else {
            ServiceRequest::NotRequired
        };
    }

    fn format_image(&mut self) {
        let Some(image) = self.image.as_mut() else {
            return;
        };

        let blocks = image.len() / SECTOR_SIZE;
        for i in 0..blocks {
            let offset = i * SECTOR_SIZE;
            let sector = &mut image[offset..offset + SECTOR_SIZE];
            sector[0..9].fill(0xff);
            sector[9..SECTOR_SIZE].fill(0xe5);
        }
    }

    fn process_write_request(&mut self, sector: u32) -> ServiceRequest {
        let Some(image) = self.image.as_mut() else {
            return ServiceRequest::NotRequired;
        };

        let offset = sector as usize * SECTOR_SIZE;

        let is_u32_max = sector == u32::MAX;
        let in_bounds = offset < image.len();

        let state = if is_u32_max {
            // Do nothing.
            // I do not know exactly why the laptop sends a write request to sector
            // 0xFFFFFFFF, but I think it is used to check the read data.
            WriteDataState::Checked
        } else if in_bounds {
            image[offset..offset + SECTOR_SIZE].copy_from_slice(&self.buffer);
            WriteDataState::Saved
        } else {
            WriteDataState::OutOfBounds
        };

        self.state = State::Write { sector, state };
        self.buffer.clear();

        return ServiceRequest::Required;
    }

    fn talk(&mut self, mut talker: Talker) {
        talker.send_bytes(self.response(), true);

        // Reset to default state after answer because disk is stateless :)
        self.reset();
    }

    fn response(&mut self) -> &[u8] {
        let Some(image) = self.image.as_mut() else {
            return &NO_DISK_RESPONSE;
        };

        match self.state {
            State::Idle => {
                panic!("Disk can't talk in idle state");
            }
            State::Initialize { .. } => {
                // TODO: What should we do?
                &OPERATION_DONE_RESPONSE
            }
            State::Identity { full } => {
                let size = if full { 56 } else { 52 };
                &self.identity[..size]
            }
            State::Read { sector } => {
                let offset = sector as usize * SECTOR_SIZE;
                if offset >= image.len() {
                    // FIXME: What is correct response for this situation?
                    &NO_DISK_RESPONSE
                } else {
                    &image[offset..offset + SECTOR_SIZE]
                }
            }
            State::Write { state, .. } => {
                if state == WriteDataState::NotAccepted {
                    panic!("Disk can't talk while waiting for data");
                }

                if state == WriteDataState::OutOfBounds {
                    // TODO: What is correct response for this situation?
                    &OPERATION_DONE_RESPONSE
                } else {
                    &OPERATION_DONE_RESPONSE
                }
            }
            State::Format => &OPERATION_DONE_RESPONSE,
        }
    }
}

impl Device for Disk {
    fn reset(&mut self) {
        self.reset();
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> ServiceRequest {
        self.process_byte(byte, eoi)
    }

    fn talk(&mut self, talker: Talker) {
        self.talk(talker)
    }
}
