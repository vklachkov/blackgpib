use std::sync::LazyLock;

use crate::gpib::GPIB;

use rppal::gpio::{Gpio, InputPin, Level, OutputPin};

static GPIO: LazyLock<Gpio> =
    LazyLock::new(|| Gpio::new().expect("should be successful on Raspberry  Pi"));

#[inline(always)]
pub fn input(gpib: GPIB) -> InputPin {
    let mut pin = GPIO
        .get(gpib.pin_number())
        .expect("pin should be used once")
        .into_input_pullup();

    pin.set_reset_on_drop(false);

    pin
}

#[inline(always)]
pub fn output(gpib: GPIB, level: Level) -> OutputPin {
    let mut pin = GPIO
        .get(gpib.pin_number())
        .expect("pin should be used once")
        .into_output();

    pin.write(level);

    pin.set_reset_on_drop(false);

    pin
}

pub fn reset_all() {
    // Set all pins to Z-state.
    for gpib in GPIB::all() {
        let mut pin = GPIO.get(gpib.pin_number()).unwrap().into_input();
        pin.set_reset_on_drop(false);
    }
}

pub fn read_data(pins: &[InputPin; 8]) -> u8 {
    let mut byte = 0u8;

    for pin in pins {
        let bit_set = pin.is_low();
        byte = (byte << 1) | (bit_set as u8);
    }

    byte
}

pub fn write_data(pins: &mut [OutputPin; 8], data: &[u8]) {
    for byte in data {
        pins[0].write(Level::from((byte >> 7) & 1));
        pins[1].write(Level::from((byte >> 6) & 1));
        pins[2].write(Level::from((byte >> 5) & 1));
        pins[3].write(Level::from((byte >> 4) & 1));
        pins[4].write(Level::from((byte >> 3) & 1));
        pins[5].write(Level::from((byte >> 2) & 1));
        pins[6].write(Level::from((byte >> 1) & 1));
        pins[7].write(Level::from((byte >> 0) & 1));
    }
}
