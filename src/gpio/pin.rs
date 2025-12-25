#![allow(unused)]

use super::{mem::GpioMem, pinout::KnownPin, types::Level};

#[derive(Debug)]
pub struct InputPin<'gpio> {
    gpio_mem: &'gpio GpioMem,
    pin: KnownPin,
}

impl<'gpio> InputPin<'gpio> {
    pub(super) unsafe fn new(gpio_mem: &'gpio GpioMem, pin: KnownPin) -> Self {
        Self { gpio_mem, pin }
    }

    /// Returns the GPIO pin number.
    #[inline]
    pub fn pin(&self) -> KnownPin {
        self.pin
    }

    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Level {
        self.gpio_mem.level(self.pin)
    }

    /// Reads the pin's logic level, and returns `true` if it's set to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn is_low(&self) -> bool {
        self.read() == Level::Low
    }

    /// Reads the pin's logic level, and returns `true` if it's set to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn is_high(&self) -> bool {
        self.read() == Level::High
    }
}

#[derive(Debug)]
pub struct OutputPin<'gpio> {
    gpio_mem: &'gpio GpioMem,
    pin: KnownPin,
}

impl<'gpio> OutputPin<'gpio> {
    pub(super) unsafe fn new(gpio_mem: &'gpio GpioMem, pin: KnownPin) -> Self {
        Self { gpio_mem, pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> KnownPin {
        self.pin
    }

    /// Reads the pin's logic level.
    fn read(&self) -> Level {
        self.gpio_mem.level(self.pin)
    }

    /// Returns `true` if the pin's output state is set to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn is_set_low(&self) -> bool {
        self.read() == Level::Low
    }

    /// Returns `true` if the pin's output state is set to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn is_set_high(&self) -> bool {
        self.read() == Level::High
    }

    /// Sets the pin's output state.
    #[inline]
    pub fn write(&self, level: Level) {
        match level {
            Level::Low => self.gpio_mem.set_low(self.pin),
            Level::High => self.gpio_mem.set_high(self.pin),
        }
    }

    /// Sets the pin's output state to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn set_low(&self) {
        self.gpio_mem.set_low(self.pin);
    }

    /// Sets the pin's output state to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn set_high(&self) {
        self.gpio_mem.set_high(self.pin);
    }
}
