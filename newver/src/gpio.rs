use std::sync::LazyLock;

use crate::gpib::GPIB;

use rppal::gpio::{Gpio, InputPin, Level, OutputPin, Result};

static GPIO: LazyLock<Gpio> =
    LazyLock::new(|| Gpio::new().expect("should be successful on Raspberry  Pi"));

#[inline(always)]
pub fn input(gpib: GPIB) -> Result<InputPin> {
    let mut pin = GPIO.get(gpib.pin_number())?.into_input_pullup();

    pin.set_reset_on_drop(false);

    Ok(pin)
}

#[inline(always)]
pub fn output(gpib: GPIB, level: Level) -> Result<OutputPin> {
    let mut pin = GPIO.get(gpib.pin_number())?.into_output();

    pin.set_reset_on_drop(false);

    match level {
        Level::Low => pin.set_low(),
        Level::High => pin.set_high(),
    }

    Ok(pin)
}

pub fn reset_all() -> Result<()> {
    // Set all pins to Z-state.
    for gpib in GPIB::all() {
        let mut pin = GPIO.get(gpib.pin_number())?.into_input();
        pin.set_reset_on_drop(false);
    }

    Ok(())
}

pub fn read_data() -> Result<u8> {
    let mut byte = 0u8;

    for pin in GPIB::data() {
        let pin = input(pin)?;
        let bit_set = pin.is_low();
        byte = (byte << 1) & (bit_set as u8);
    }

    Ok(byte)
}
