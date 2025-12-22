use crate::{
    gpib_pinout::GPIBPin,
    gpio::{Gpio, InputPin, OutputPin},
};

pub fn reset_all_pins(gpio: &Gpio) {
    let gpib_pins = const { GPIBPin::all() };

    // Set all pins to Z-state.
    for gpib_pin in gpib_pins {
        // SAFETY: TODO
        unsafe { gpio.get(gpib_pin.pin_number()) }.into_input();
    }
}

#[allow(unused)]
pub struct CommonPins<'gpio> {
    pub dc: OutputPin<'gpio>,
    pub te: OutputPin<'gpio>,
    pub pe: OutputPin<'gpio>,

    pub atn: InputPin<'gpio>,

    pub srq: OutputPin<'gpio>,
    pub ren: InputPin<'gpio>,
    pub ifc: InputPin<'gpio>,
}

impl<'gpio> CommonPins<'gpio> {
    pub fn new(gpio: &'gpio Gpio) -> Self {
        Self {
            dc: unsafe { gpio.get(GPIBPin::DC.pin_number()) }.into_output_high(),
            te: unsafe { gpio.get(GPIBPin::TE.pin_number()) }.into_output_high(),
            pe: unsafe { gpio.get(GPIBPin::PE.pin_number()) }.into_output_high(),

            atn: unsafe { gpio.get(GPIBPin::ATN.pin_number()) }.into_input_pullup(),

            srq: unsafe { gpio.get(GPIBPin::SRQ.pin_number()) }.into_output_high(),
            ren: unsafe { gpio.get(GPIBPin::REN.pin_number()) }.into_input_pullup(),
            ifc: unsafe { gpio.get(GPIBPin::IFC.pin_number()) }.into_input_pullup(),
        }
    }
}
