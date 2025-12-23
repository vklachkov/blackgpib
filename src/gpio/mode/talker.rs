use crate::gpio::GpioMem;

pub struct GpioMode<'gpio> {
    gpio: &'gpio mut GpioMem,
}

impl<'gpio> GpioMode<'gpio> {
    pub(in crate::gpio) fn new(gpio: &'gpio mut GpioMem) -> Self {
        Self { gpio }
    }
}
