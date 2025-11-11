mod identity;
mod request;

use crate::{debug, devices::Device, error, info, talker::Talker, trace};

use identity::DiskIdentity;
use request::{Request, RequestCode};

const SECTOR_SIZE: usize = 512;
const LOGICAL_SECTOR_SIZE: usize = 504;

const OUT_OF_BOUNDS_RESPONSE: [u8; 7] = [0x6b, 0, 0, 0, 0, 0, 0];
const WRITE_SUCCESSFUL_RESPONSE: [u8; 7] = [0; 7];

#[derive(Clone, Debug, Default)]
enum State {
    #[default]
    Idle,
    Identity {
        full: bool,
    },
    Read {
        sector: u32,
    },
    Write {
        sector: u32,
        state: WriteDataState,
    },
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
    image: Vec<u8>,
    buffer: Vec<u8>,
    state: State,
}

impl Disk {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            identity: Self::identity(name, 0xFFFF, 0xFFFF, 0xFFFF).into_bytes(),
            image: Vec::new(),
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

    pub fn use_image(&mut self, mut image: Vec<u8>, superblock_id: u16, bitmap_block_id: u16) {
        // Make the image size a multiple of the sector size.
        let sector_remainder = image.len() % SECTOR_SIZE;
        if sector_remainder != 0 {
            let padding = SECTOR_SIZE - sector_remainder;
            image.extend(std::iter::repeat(0u8).take(padding));
        }

        self.image = image;

        let sector_count = (self.image.len() / SECTOR_SIZE) as u16;
        self.identity = Self::identity(&self.name, sector_count, superblock_id, bitmap_block_id).into_bytes();
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.state = State::Idle;
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool {
        self.buffer.push(byte);

        if eoi {
            self.process_buffer()
        } else {
            // Data is being received, service request is not needed.
            false
        }
    }

    fn process_buffer(&mut self) -> bool {
        match self.state {
            State::Idle => {
                return self.process_new_request();
            }
            State::Identity { .. } => {
                panic!("unexpected data received");
            }
            State::Read { .. } => {
                panic!("unexpected data received");
            }
            State::Write { sector, state } => {
                if state != WriteDataState::NotAccepted {
                    panic!("unexpected data received")
                }

                return self.process_write_request(sector);
            }
        }
    }

    fn process_new_request(&mut self) -> bool {
        let raw = self.buffer.as_slice();

        trace!("Parse disk request {raw:02x?}...");

        let req = match Request::try_from(raw) {
            Ok(value) => value,
            Err(err) => {
                error!("Failed to parse request: {err}");
                return false;
            }
        };

        debug!("Received disk request {req:?}");

        self.state = match req.code {
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
            _ => panic!("Unexpected request {req:?}"),
        };

        req.code == RequestCode::Read
    }

    fn process_write_request(&mut self, sector: u32) -> bool {
        let offset = sector as usize * SECTOR_SIZE;

        let is_u32_max = sector == u32::MAX;
        let in_bounds = offset < self.image.len();

        let state = if is_u32_max {
            // Do nothing.
            // I do not know exactly why the laptop sends a write request to sector
            // 0xFFFFFFFF, but I think it is used to check the read data.
            WriteDataState::Checked
        } else if in_bounds {
            self.image[offset..offset + SECTOR_SIZE].copy_from_slice(&self.buffer);
            WriteDataState::Saved
        } else {
            WriteDataState::OutOfBounds
        };

        self.state = State::Write { sector, state };

        return true;
    }

    fn talk(&mut self, mut talker: Talker) {
        talker.send_bytes(self.response(), true);
        self.reset();
    }

    fn response(&mut self) -> &[u8] {
        match self.state {
            State::Idle => {
                panic!("Disk can't talk in idle state");
            }
            State::Identity { full } => {
                let size = if full { 56 } else { 52 };
                &self.identity[..size]
            }
            State::Read { sector } => {
                let offset = sector as usize * SECTOR_SIZE;
                if offset >= self.image.len() {
                    &OUT_OF_BOUNDS_RESPONSE
                } else {
                    &self.image[offset..offset + SECTOR_SIZE]
                }
            }
            State::Write { state, .. } => {
                if state == WriteDataState::NotAccepted {
                    panic!("Disk can't talk while waiting for data");
                }

                if state == WriteDataState::OutOfBounds {
                    &OUT_OF_BOUNDS_RESPONSE
                } else {
                    &WRITE_SUCCESSFUL_RESPONSE
                }
            }
        }
    }
}

impl Device for Disk {
    fn reset(&mut self) {
        self.reset();
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool {
        self.process_byte(byte, eoi)
    }

    fn talk(&mut self, talker: Talker) {
        self.talk(talker)
    }
}
