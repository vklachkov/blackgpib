mod gpib;
mod gpio;

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use rppal::gpio::{Level, Result};

use crate::gpib::GPIB;

const ADDRESS: u8 = 5; // 4 for HDD 10MB, 5 is ???, 6 for Floppy 5.25.

struct ListenerStateMachine {
    current_state: ListenerState,
    buffer: Vec<u8>,
}

#[derive(Clone, Copy)]
enum ListenerState {
    Idle,
    Addressed,
    Active,
}

impl ListenerStateMachine {
    fn idle() -> Self {
        Self {
            current_state: ListenerState::Idle,
            buffer: Vec::with_capacity(16),
        }
    }

    /// Return bytes on unlisten.
    fn process(&mut self, byte: u8, is_command: bool) -> Option<Vec<u8>> {
        match self.current_state {
            ListenerState::Idle => {
                if is_command && Self::is_mla(byte) {
                    println!("Idle -> Addressed");
                    self.current_state = ListenerState::Addressed;
                }
            }
            ListenerState::Addressed => {
                if is_command {
                    if Self::is_unlisten_command(byte) || Self::is_mta_command(byte) {
                        println!("Addressed -> Idle");
                        self.current_state = ListenerState::Idle;
                    }
                } else {
                    println!("Addressed -> Active");
                    self.current_state = ListenerState::Active;
                    self.buffer.push(byte);
                }
            }
            ListenerState::Active => {
                if is_command {
                    if Self::is_unlisten_command(byte) || Self::is_mta_command(byte) {
                        println!("Active -> Idle");
                        self.current_state = ListenerState::Idle;

                        let read = self.buffer.clone();
                        self.buffer.clear();

                        return Some(read);
                    }
                } else {
                    println!("Save byte {byte:#02x}");
                    self.buffer.push(byte);
                }
            }
        };

        None
    }

    fn is_mla(byte: u8) -> bool {
        (byte & 0b0111_1111) == (0b0010_0000 | ADDRESS)
    }

    fn is_unlisten_command(byte: u8) -> bool {
        (byte & 0b0111_1111) == 0b0011_1111
    }

    fn is_mta_command(byte: u8) -> bool {
        (byte & 0b0111_1111) == (0b0100_0000 | ADDRESS)
    }
}

fn main() -> Result<()> {
    let start = Instant::now();

    gpio::reset_all()?;

    let _dc = gpio::output(GPIB::DC, Level::High)?;
    let _te = gpio::output(GPIB::TE, Level::Low)?;
    let _pe = gpio::output(GPIB::PE, Level::Low)?;

    let atn = gpio::input(GPIB::ATN)?;
    let eoi = gpio::input(GPIB::EOI)?;

    let dav = gpio::input(GPIB::DAV)?;
    let mut srq = gpio::output(GPIB::SRQ, Level::High)?;
    let mut ndac = gpio::output(GPIB::NDAC, Level::Low)?;
    let mut nrfd = gpio::output(GPIB::NRFD, Level::Low)?;

    let mut listener = ListenerStateMachine::idle();

    loop {
        // Ready for data.
        ndac.set_low();
        nrfd.set_high();

        // Wait laptop.
        // println!("Wait DAV low");
        while dav.is_high() {
            sleep(Duration::from_micros(100));
        }
        // println!("DAV is low");

        // Not ready for data.
        nrfd.set_low();

        // Read all.
        let atn = atn.is_low() as u8;
        let eoi = eoi.is_low() as u8;
        let byte = gpio::read_data()?;

        println!(
            "{}ms GPIB: ATN={atn} EOI={eoi} BYTE={byte:#02x} ({byte:#08b})",
            (Instant::now() - start).as_millis()
        );

        let received = listener.process(byte, atn == 1);
        if let Some(received) = received {
            println!("Listener completed: {received:02x?}");
        }

        // Notify that we read byte.
        ndac.set_high();

        // Wait laptop.
        // println!("Wait DAV high");
        while dav.is_low() {
            sleep(Duration::from_micros(100));
        }
        // println!("DAV is high");
    }
}
