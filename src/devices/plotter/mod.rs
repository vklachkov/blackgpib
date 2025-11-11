use std::{
    fs,
    io::{BufWriter, Write},
};

use crate::{debug, talker::Talker};

use super::device::{Device, ServiceRequest};

pub struct GenericPlotter {
    f: BufWriter<fs::File>,
}

impl GenericPlotter {
    pub fn new() -> Self {
        Self {
            f: BufWriter::with_capacity(
                128,
                fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open("plotter.hpgl")
                    .unwrap(),
            ),
        }
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) {
        if eoi {
            _ = self.f.write(&[byte, b'\r', b'\n']);
        } else {
            _ = self.f.write(&[byte]);
        }

        _ = self.f.flush();
    }
}

impl Device for GenericPlotter {
    fn reset(&mut self) {
        // Do nothing. The printer has no state.
    }

    fn process_byte(&mut self, byte: u8, eoi: bool) -> ServiceRequest {
        self.process_byte(byte, eoi);
        ServiceRequest::NotRequired
    }

    fn talk(&mut self, _talker: Talker) {
        // Do nothing. The printer cannot talk.
    }
}
