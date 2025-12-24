use crate::gpio::{
    GpioMem, InputPin, OutputPin,
    pin::Pin,
    pinout::KnownPin,
    types::{Bias, Mode, PinMask, PinModesRegs},
};

macro_rules! get_pin {
    ($ident:ident, $pin:expr, $output:ident<$lt:lifetime>) => {
        #[inline(always)]
        pub fn $ident(&$lt self) -> $output<$lt> {
            // SAFETY: pin configured properly.
            unsafe { $output::new(Pin::new(&self.gpio_mem, $pin)) }
        }
    };
}

pub struct GpioMode<'gpio> {
    gpio_mem: &'gpio mut GpioMem,
}

impl<'gpio> GpioMode<'gpio> {
    pub(in crate::gpio) fn new(gpio_mem: &'gpio mut GpioMem) -> Self {
        gpio_mem.write_pins_modes(const { Self::pin_modes() });

        gpio_mem.set_high(KnownPin::DC as _);
        gpio_mem.set_low(KnownPin::TE as _);
        gpio_mem.set_pins_high(const { Self::output_pins_mask() });

        Self { gpio_mem }
    }

    const fn pin_modes() -> PinModesRegs {
        let mut regs = PinModesRegs::new();

        regs.set(KnownPin::DC, Mode::Output);
        regs.set(KnownPin::TE, Mode::Output);
        regs.set(KnownPin::PE, Mode::Output);

        regs.set(KnownPin::ATN, Mode::Input);
        regs.set(KnownPin::SRQ, Mode::Output);
        regs.set(KnownPin::REN, Mode::Input);
        regs.set(KnownPin::IFC, Mode::Input);
        regs.set(KnownPin::EOI, Mode::Input);
        regs.set(KnownPin::DAV, Mode::Input);

        regs.set(KnownPin::NDAC, Mode::Output);
        regs.set(KnownPin::NRFD, Mode::Output);

        let data_pins = KnownPin::data();

        let mut i = 0;
        while i < data_pins.len() {
            regs.set(data_pins[i], Mode::Input);
            i += 1;
        }

        regs
    }

    const fn output_pins_mask() -> PinMask {
        let mut mask = PinMask::new();

        mask.set(KnownPin::PE);

        mask.set(KnownPin::SRQ);

        mask.set(KnownPin::NDAC);
        mask.set(KnownPin::NRFD);

        mask
    }

    get_pin!(atn, KnownPin::ATN, InputPin<'gpio>);
    get_pin!(srq, KnownPin::SRQ, OutputPin<'gpio>);
    get_pin!(eoi, KnownPin::EOI, InputPin<'gpio>);
    get_pin!(dav, KnownPin::DAV, InputPin<'gpio>);
    get_pin!(ndac, KnownPin::NDAC, OutputPin<'gpio>);
    get_pin!(nrfd, KnownPin::NRFD, OutputPin<'gpio>);

    pub fn read_dio(&self) -> u8 {
        let levels = self.gpio_mem.levels();
        let levels_inv = !levels;

        let mut data = 0;
        let data_pins = const { KnownPin::data() };

        for i in 0..data_pins.len() {
            data |= (levels_inv >> data_pins[i] as u8 & 0b1) << i;
        }

        data as u8
    }
}
