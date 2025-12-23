pub mod listener;
pub mod talker;

use crate::gpio::{
    GpioMem,
    pinout::KnownPin,
    types::{Bias, Level, Mode},
};

pub(super) fn prepare_common_pins(gpio_mem: &GpioMem) {
    let output_pins = [
        (KnownPin::DC, Level::High),
        (KnownPin::TE, Level::Low),
        (KnownPin::PE, Level::High),
        (KnownPin::SRQ, Level::High),
    ];

    for (pin, level) in output_pins {
        gpio_mem.set_mode(pin as u8, Mode::Output);

        if level == Level::High {
            gpio_mem.set_high(pin as _);
        } else {
            gpio_mem.set_low(pin as _);
        }
    }

    let input_pins = [KnownPin::ATN, KnownPin::REN, KnownPin::IFC];

    for (pin, level) in output_pins {
        gpio_mem.set_mode(pin as u8, Mode::Input);
        gpio_mem.set_bias(pin as u8, Bias::PullUp);
    }
}
