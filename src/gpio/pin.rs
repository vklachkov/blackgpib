use super::{
    mem::GpioMem,
    pinout::KnownPin,
    types::{Bias, Level, Mode},
};

/// Unconfigured GPIO pin.
///
/// `Pin`s are constructed by retrieving them using [`Gpio::get`].
///
/// An unconfigured `Pin` can be used to read the pin's mode and logic level.
/// Converting the `Pin` to an [`InputPin`], [`OutputPin`] or [`IoPin`] through the
/// various `into_` methods available on `Pin` configures the appropriate mode, and
/// provides access to additional methods relevant to the selected pin mode.
///
/// The `embedded-hal` trait implementations for `Pin` can be enabled by specifying
/// the optional `hal` feature in the dependency declaration for the `rppal` crate.
///
/// [`Gpio::get`]: struct.Gpio.html#method.get
/// [`InputPin`]: struct.InputPin.html
/// [`OutputPin`]: struct.OutputPin.html
/// [`IoPin`]: struct.IoPin.html
#[derive(Debug)]
pub(super) struct Pin<'gpio> {
    gpio_mem: &'gpio GpioMem,
    pin: KnownPin,
}

impl<'gpio> Pin<'gpio> {
    pub(super) fn new(gpio_mem: &'gpio GpioMem, pin: KnownPin) -> Pin<'gpio> {
        Pin { gpio_mem, pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM GPIO numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> KnownPin {
        self.pin
    }

    /// Returns the pin's mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.gpio_mem.mode(self.pin as _)
    }

    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Level {
        unsafe { self.gpio_mem.level(self.pin as _) }
    }

    #[inline]
    pub(crate) fn set_mode(&self, mode: Mode) {
        self.gpio_mem.set_mode(self.pin as _, mode);
    }

    #[inline]
    pub(crate) fn set_bias(&self, bias: Bias) {
        self.gpio_mem.set_bias(self.pin as _, bias);
    }

    #[inline]
    pub(crate) fn set_low(&self) {
        self.gpio_mem.set_low(self.pin as _);
    }

    #[inline]
    pub(crate) fn set_high(&self) {
        self.gpio_mem.set_high(self.pin as _);
    }

    #[inline]
    pub(crate) fn write(&self, level: Level) {
        match level {
            Level::Low => self.set_low(),
            Level::High => self.set_high(),
        };
    }
}

/// GPIO pin configured as input.
///
/// `InputPin`s are constructed by converting a [`Pin`] using [`Pin::into_input`],
/// [`Pin::into_input_pullup`] or [`Pin::into_input_pulldown`]. The pin's mode is
/// automatically set to [`Mode::Input`].
///
/// An `InputPin` can be used to read a pin's logic level, or (a)synchronously poll for
/// interrupt trigger events.
///
/// The `embedded-hal` trait implementations for `InputPin` can be enabled by specifying
/// the optional `hal` feature in the dependency declaration for the `rppal` crate.
///
/// [`Pin`]: struct.Pin.html
/// [`Mode::Input`]: enum.Mode.html#variant.Input
/// [`Pin::into_input`]: struct.Pin.html#method.into_input
/// [`Pin::into_input_pullup`]: struct.Pin.html#method.into_input_pullup
/// [`Pin::into_input_pulldown`]: struct.Pin.html#method.into_input_pulldown
#[derive(Debug)]
pub struct InputPin<'gpio> {
    pub(super) pin: Pin<'gpio>,
}

impl<'gpio> InputPin<'gpio> {
    pub(super) unsafe fn new(pin: Pin<'gpio>) -> Self {
        Self { pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> KnownPin {
        self.pin.pin
    }

    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Level {
        self.pin.read()
    }

    /// Reads the pin's logic level, and returns `true` if it's set to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn is_low(&self) -> bool {
        self.pin.read() == Level::Low
    }

    /// Reads the pin's logic level, and returns `true` if it's set to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn is_high(&self) -> bool {
        self.pin.read() == Level::High
    }

    /// Configures the built-in pull-up/pull-down resistors.
    #[inline]
    pub fn set_bias(&mut self, bias: Bias) {
        self.pin.set_bias(bias);
    }
}

/// GPIO pin configured as output.
///
/// `OutputPin`s are constructed by converting a [`Pin`] using [`Pin::into_output`],
/// [`Pin::into_output_low`] or [`Pin::into_output_high`]. The pin's mode is automatically set to
/// [`Mode::Output`].
///
/// An `OutputPin` can be used to change a pin's output state.
///
/// The `embedded-hal` trait implementations for `OutputPin` can be enabled by specifying
/// the optional `hal` feature in the dependency declaration for the `rppal` crate.
///
/// [`Pin`]: struct.Pin.html
/// [`Mode::Output`]: enum.Mode.html#variant.Output
/// [`Pin::into_output_low`]: struct.Pin.html#method.into_output_low
/// [`Pin::into_output_high`]: struct.Pin.html#method.into_output_high
#[derive(Debug)]
pub struct OutputPin<'gpio> {
    pin: Pin<'gpio>,
}

impl<'gpio> OutputPin<'gpio> {
    pub(super) unsafe fn new(pin: Pin<'gpio>) -> Self {
        OutputPin { pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> KnownPin {
        self.pin.pin
    }

    /// Returns `true` if the pin's output state is set to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn is_set_low(&self) -> bool {
        self.pin.read() == Level::Low
    }

    /// Returns `true` if the pin's output state is set to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn is_set_high(&self) -> bool {
        self.pin.read() == Level::High
    }

    /// Sets the pin's output state.
    #[inline]
    pub fn write(&self, level: Level) {
        self.pin.write(level)
    }

    /// Sets the pin's output state to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn set_low(&self) {
        self.pin.set_low()
    }

    /// Sets the pin's output state to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn set_high(&self) {
        self.pin.set_high()
    }
}
