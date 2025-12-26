use crate::gpio::{
    GpioMem, InputPin, OutputPin,
    pinout::KnownPin,
    types::{Mode, PinMask, PinModesRegs},
};

macro_rules! get_pin {
    ($ident:ident, $pin:expr, $output:ident<$lt:lifetime>) => {
        #[inline(always)]
        pub fn $ident(&$lt self) -> $output<$lt> {
            // SAFETY: pin configured properly.
            unsafe { $output::new(&self.gpio_mem, $pin) }
        }
    };
}

pub struct GpioMode<'gpio> {
    gpio_mem: &'gpio mut GpioMem,
}

impl<'gpio> GpioMode<'gpio> {
    pub(in crate::gpio) fn new(gpio_mem: &'gpio mut GpioMem) -> Self {
        gpio_mem.write_pins_modes(const { Self::pin_modes() });

        gpio_mem.set_pins_high(const { Self::output_pins_mask() });

        gpio_mem.set_low(KnownPin::DC as _);
        gpio_mem.set_high(KnownPin::TE as _);

        Self { gpio_mem }
    }

    const fn pin_modes() -> PinModesRegs {
        let mut regs = PinModesRegs::new();

        regs.set(KnownPin::DC, Mode::Output);
        regs.set(KnownPin::TE, Mode::Output);
        regs.set(KnownPin::PE, Mode::Output);

        regs.set(KnownPin::ATN, Mode::Output);
        regs.set(KnownPin::SRQ, Mode::Input);
        regs.set(KnownPin::REN, Mode::Output);
        regs.set(KnownPin::IFC, Mode::Output);
        regs.set(KnownPin::EOI, Mode::Output);
        regs.set(KnownPin::DAV, Mode::Output);

        regs.set(KnownPin::NDAC, Mode::Input);
        regs.set(KnownPin::NRFD, Mode::Input);

        let data_pins = KnownPin::data();

        let mut i = 0;
        while i < data_pins.len() {
            regs.set(data_pins[i], Mode::Output);
            i += 1;
        }

        regs
    }

    const fn output_pins_mask() -> PinMask {
        let mut mask = PinMask::new();

        mask.set(KnownPin::PE);

        mask.set(KnownPin::ATN);
        mask.set(KnownPin::REN);
        mask.set(KnownPin::IFC);
        mask.set(KnownPin::EOI);
        mask.set(KnownPin::DAV);

        let data_pins = KnownPin::data();

        let mut i = 0;
        while i < data_pins.len() {
            mask.set(data_pins[i]);
            i += 1;
        }

        mask
    }

    get_pin!(atn, KnownPin::ATN, OutputPin<'gpio>);
    get_pin!(srq, KnownPin::SRQ, InputPin<'gpio>);
    get_pin!(eoi, KnownPin::EOI, OutputPin<'gpio>);
    get_pin!(dav, KnownPin::DAV, OutputPin<'gpio>);
    get_pin!(ndac, KnownPin::NDAC, InputPin<'gpio>);
    get_pin!(nrfd, KnownPin::NRFD, InputPin<'gpio>);

    pub fn write_dio(&self, byte: u8) {
        for (i, pin) in KnownPin::data().into_iter().enumerate() {
            let bit = byte >> i & 0b1;
            if bit == 1 {
                self.gpio_mem.set_low(pin as _);
            } else {
                self.gpio_mem.set_high(pin as _);
            }
        }
    }
}
