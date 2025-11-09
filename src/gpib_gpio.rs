use std::sync::LazyLock;

use crate::gpib_pinout::GPIBPin;

use rppal::gpio::{Gpio, InputPin, Level, OutputPin};

static GPIO: LazyLock<Gpio> = LazyLock::new(|| Gpio::new().expect("failed to initialize Raspberry Pi GPIO"));

#[inline(always)]
pub fn input(pin: GPIBPin) -> InputPin {
    let mut pin = GPIO
        .get(pin.pin_number())
        .expect("pin should be used once")
        .into_input_pullup();

    pin.set_reset_on_drop(false);

    pin
}

#[inline(always)]
pub fn output(pin: GPIBPin, level: Level) -> OutputPin {
    let mut pin = GPIO
        .get(pin.pin_number())
        .expect("pin should be used once")
        .into_output();

    pin.write(level);

    pin.set_reset_on_drop(false);

    pin
}

pub fn reset_all() {
    // Set all pins to Z-state.
    for gpib in GPIBPin::all() {
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

pub fn write_data(pins: &mut [OutputPin; 8], byte: u8) {
    write_bool(&mut pins[0], (byte >> 7) & 1);
    write_bool(&mut pins[1], (byte >> 6) & 1);
    write_bool(&mut pins[2], (byte >> 5) & 1);
    write_bool(&mut pins[3], (byte >> 4) & 1);
    write_bool(&mut pins[4], (byte >> 3) & 1);
    write_bool(&mut pins[5], (byte >> 2) & 1);
    write_bool(&mut pins[6], (byte >> 1) & 1);
    write_bool(&mut pins[7], (byte >> 0) & 1);
}

fn write_bool(pin: &mut OutputPin, value: u8) {
    if value == 0 {
        pin.set_high();
    } else {
        pin.set_low();
    }
}
