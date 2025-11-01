use std::sync::LazyLock;

use crate::gpib::GPIB;

use rppal::gpio::{Gpio, InputPin, Level, OutputPin, Result};

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

pub fn reset_all() -> Result<()> {
    // Set all pins to Z-state.
    for gpib in GPIB::all() {
        let mut pin = GPIO.get(gpib.pin_number())?.into_input();
        pin.set_reset_on_drop(false);
    }

    Ok(())
}

pub fn read_data(pins: &[InputPin; 8]) -> Result<u8> {
    let mut byte = 0u8;

    for pin in pins {
        let bit_set = pin.is_low();
        byte = (byte << 1) | (bit_set as u8);
    }

    Ok(byte)
}
