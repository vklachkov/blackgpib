use crate::devices::Device;

mod identity;
mod request;

const SECTOR_SIZE: usize = 512;

const OUT_OF_DISK_RESPONSE: [u8; 7] = [0x6b, 0, 0, 0, 0, 0, 0];
const WRITE_SUCCESSFUL_RESPONSE: [u8; 7] = [0; 7];

pub struct Disk {
    name: String,
    image: Vec<u8>,
    buffer: Vec<u8>,
}

impl Disk {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            image: Vec::new(),
            buffer: Vec::with_capacity(SECTOR_SIZE),
        }
    }

    pub fn use_image(&mut self, mut image: Vec<u8>) {
        // Make the image size a multiple of the sector size.
        let sector_remainder = image.len() % SECTOR_SIZE;
        if sector_remainder != 0 {
            let padding = SECTOR_SIZE - sector_remainder;
            image.extend(std::iter::repeat(0u8).take(padding));
        }

        self.image = image;
    }
}

impl Device for Disk {
    fn reset(&mut self) {
        todo!()
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> bool {
        todo!()
    }

    fn talk(&mut self, talker: crate::talker::Talker) {
        todo!()
    }
}
