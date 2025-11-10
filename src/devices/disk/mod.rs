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
        sector_number: u32,
    },
    Write {
        sector: u32,
        data_received: bool,
    },
}

pub struct Disk {
    identity: DiskIdentity,
    identity_bytes: [u8; 56],
    image: Vec<u8>,
    buffer: Vec<u8>,
    state: State,
}

impl Disk {
    pub fn new(name: &str) -> Self {
        let identity = Self::create_identity(name);

        Self {
            identity: identity,
            identity_bytes: identity.into_bytes(),
            image: Vec::new(),
            buffer: Vec::with_capacity(SECTOR_SIZE),
            state: State::Idle,
        }
    }

    fn create_identity(name: &str) -> DiskIdentity {
        assert!(name.is_ascii(), "Device name must be ASCII");

        let mut device_name = [b' '; 32];
        let name_len = name.len().min(device_name.len());
        device_name[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);

        DiskIdentity {
            sector_size: SECTOR_SIZE as _,
            logical_sector_size: LOGICAL_SECTOR_SIZE as _,
            sector_count: 0xFFFF,
            drive_ready: true,
            bitmap_block_id: 0xFFFF,
            superblock_id: 0xFFFF,
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

        self.identity.sector_count = (self.image.len() / SECTOR_SIZE) as u16;
        self.identity.superblock_id = superblock_id;
        self.identity.bitmap_block_id = bitmap_block_id;
        self.identity_bytes = self.identity.into_bytes();
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
            State::Idle => self.process_request(),
            State::Identity { .. } => panic!("unexpected data received"),
            State::Read { .. } => panic!("unexpected data received"),
            State::Write { sector, data_received } => {
                if data_received {
                    panic!("unexpected data received")
                } else {
                    self.state = State::Write {
                        sector,
                        data_received: true,
                    };
                    true
                }
            }
        }
    }

    fn process_request(&mut self) -> bool {
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
                // Sometimes the Compass may request 54 bytes of identifier,
                // and in this case it is necessary to reply with exactly 52 bytes.
                full: req.data_size == 56,
            },
            RequestCode::Read => State::Read {
                sector_number: req.sector,
            },
            RequestCode::Write => State::Write {
                sector: req.sector,
                data_received: false,
            },
            _ => panic!("Unexpected request {req:?}"),
        };

        req.code == RequestCode::Read
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
                &self.identity_bytes[..size]
            }
            State::Read { sector_number } => {
                let offset = sector_number as usize * SECTOR_SIZE;
                if offset >= self.image.len() {
                    &OUT_OF_BOUNDS_RESPONSE
                } else {
                    &self.image[offset..offset + SECTOR_SIZE]
                }
            }
            State::Write {
                sector: sector_number,
                data_received,
            } => {
                if !data_received {
                    panic!("Disk can't talk while waiting for data");
                }

                let offset = sector_number as usize * SECTOR_SIZE;
                if offset >= self.image.len() {
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
