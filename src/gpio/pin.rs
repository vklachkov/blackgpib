use super::{Bias, Level, Mode, bcm::GpioMem};

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
pub struct Pin<'gpio> {
    gpio_mem: &'gpio GpioMem,
    pub(crate) pin: u8,
}

impl<'gpio> Pin<'gpio> {
    #[inline]
    pub(crate) unsafe fn new(pin: u8, gpio_mem: &'gpio GpioMem) -> Pin<'gpio> {
        Pin { pin, gpio_mem }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM GPIO numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> u8 {
        self.pin
    }

    /// Returns the pin's mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.gpio_mem.mode(self.pin)
    }

    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Level {
        self.gpio_mem.level(self.pin)
    }

    /// Consumes the `Pin` and returns an [`InputPin`]. Sets the mode to [`Input`]
    /// and disables the pin's built-in pull-up/pull-down resistors.
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    #[inline]
    pub fn into_input(self) -> InputPin<'gpio> {
        InputPin::new(self, Bias::Off)
    }

    /// Consumes the `Pin` and returns an [`InputPin`]. Sets the mode to [`Input`]
    /// and enables the pin's built-in pull-down resistor.
    ///
    /// The pull-down resistor is disabled when `InputPin` goes out of scope if [`reset_on_drop`]
    /// is set to `true` (default).
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    /// [`reset_on_drop`]: struct.InputPin.html#method.set_reset_on_drop
    #[inline]
    pub fn into_input_pulldown(self) -> InputPin<'gpio> {
        InputPin::new(self, Bias::PullDown)
    }

    /// Consumes the `Pin` and returns an [`InputPin`]. Sets the mode to [`Input`]
    /// and enables the pin's built-in pull-up resistor.
    ///
    /// The pull-up resistor is disabled when `InputPin` goes out of scope if [`reset_on_drop`]
    /// is set to `true` (default).
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    /// [`reset_on_drop`]: struct.InputPin.html#method.set_reset_on_drop
    #[inline]
    pub fn into_input_pullup(self) -> InputPin<'gpio> {
        InputPin::new(self, Bias::PullUp)
    }

    /// Consumes the `Pin` and returns an [`OutputPin`]. Sets the mode to [`Mode::Output`]
    /// and leaves the logic level unchanged.
    #[inline]
    pub fn into_output(self, level: Level) -> OutputPin<'gpio> {
        OutputPin::new(self, level)
    }

    /// Consumes the `Pin` and returns an [`OutputPin`]. Changes the logic level to
    /// [`Level::Low`] and then sets the mode to [`Mode::Output`].
    #[inline]
    pub fn into_output_low(self) -> OutputPin<'gpio> {
        OutputPin::new(self, Level::Low)
    }

    /// Consumes the `Pin` and returns an [`OutputPin`]. Changes the logic level to
    /// [`Level::High`] and then sets the mode to [`Mode::Output`].
    #[inline]
    pub fn into_output_high(self) -> OutputPin<'gpio> {
        OutputPin::new(self, Level::High)
    }

    #[inline]
    pub(crate) fn set_mode(&self, mode: Mode) {
        self.gpio_mem.set_mode(self.pin, mode);
    }

    #[inline]
    pub(crate) fn set_bias(&self, bias: Bias) {
        self.gpio_mem.set_bias(self.pin, bias);
    }

    #[inline]
    pub(crate) fn set_low(&self) {
        self.gpio_mem.set_low(self.pin);
    }

    #[inline]
    pub(crate) fn set_high(&self) {
        self.gpio_mem.set_high(self.pin);
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
    pub(crate) pin: Pin<'gpio>,
}

impl InputPin<'_> {
    pub(crate) fn new(pin: Pin, bias: Bias) -> InputPin {
        pin.set_mode(Mode::Input);
        pin.set_bias(bias);

        InputPin { pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> u8 {
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

impl OutputPin<'_> {
    pub(crate) fn new(pin: Pin, level: Level) -> OutputPin {
        pin.set_mode(Mode::Output);
        pin.write(level);

        OutputPin { pin }
    }

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> u8 {
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
