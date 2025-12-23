// use std::time::Instant;

// use crate::{
//     common::{CommonPins, reset_all_pins},
//     listener::Listener,
//     gpio::Gpio,
// };

pub struct BusSniffer {
    file: memmap2::MmapMut,
}

impl BusSniffer {
    pub fn new(file: memmap2::MmapMut) -> Self {
        Self { file }
    }

    pub fn start(mut self) {
        // let start_time = Instant::now();

        // let gpio = unsafe { Gpio::new() }.unwrap();
        // reset_all_pins(&gpio);

        // let common_pins = CommonPins::new(&gpio);
        // let sniffer = Listener::new(&gpio, &common_pins);

        // let mut offset = 0usize;
        // loop {
        //     const ENTRY_SIZE: usize = 5;

        //     if (offset + ENTRY_SIZE * 2) >= self.file.len() {
        //         break;
        //     }

        //     let byte = sniffer.sniff_byte();
        //     let timestamp = start_time.elapsed().as_millis();

        //     self.file[offset + 0] = ((timestamp & 0x0000FF) >> 00) as u8;
        //     self.file[offset + 1] = ((timestamp & 0x00FF00) >> 08) as u8;
        //     self.file[offset + 2] = ((timestamp & 0xFF0000) >> 16) as u8;
        //     self.file[offset + 3] = byte.value;
        //     self.file[offset + 4] = ((byte.atn as u8) << 1) | (byte.eoi as u8);
        //     offset += ENTRY_SIZE;

        //     // End marker.
        //     self.file[offset..offset + ENTRY_SIZE].fill(0x00);
        // }
    }
}
