
#![allow(unused)]

//! Interface for the GPIO peripheral.
//!
//! To ensure fast performance, RPPAL controls the GPIO peripheral by directly
//! accessing the registers through either `/dev/gpiomem` or `/dev/mem`.
//!
//! ## Pins
//!
//! GPIO pins are retrieved from a [`Gpio`] instance by their BCM GPIO number by calling
//! [`Gpio::get`]. The returned unconfigured [`Pin`] can be used to read the pin's
//! mode and logic level. Converting the [`Pin`] to an [`InputPin`], [`OutputPin`] or
//! [`IoPin`] through the various `into_` methods available on [`Pin`] configures the
//! appropriate mode, and provides access to additional methods relevant to the selected pin mode.
//!
//! Retrieving a GPIO pin with [`Gpio::get`] grants access to the pin through an owned [`Pin`]
//! instance. If the pin is already in use, or the GPIO peripheral doesn't expose a pin with
//! the specified number, [`Gpio::get`] returns `Err(`[`Error::PinNotAvailable`]`)`. After a [`Pin`]
//! (or a derived [`InputPin`], [`OutputPin`] or [`IoPin`]) goes out of scope, it can be
//! retrieved again through another [`Gpio::get`] call.
//!
//! By default, pins are reset to their original state when they go out of scope.
//! Use [`InputPin::set_reset_on_drop(false)`], [`OutputPin::set_reset_on_drop(false)`]
//! or [`IoPin::set_reset_on_drop(false)`], respectively, to disable this behavior.
//! Note that `drop` methods aren't called when a process is abnormally terminated (for
//! instance when a `SIGINT` signal isn't caught).
//!
//! ## Examples
//!
//! Basic example:
//!
//! ```
//! use std::thread;
//! use std::time::Duration;
//!
//! use rppal::gpio::Gpio;
//!
//! # fn main() -> rppal::gpio::Result<()> {
//! let gpio = Gpio::new()?;
//! let mut pin = gpio.get(23)?.into_output();
//!
//! pin.set_high();
//! thread::sleep(Duration::from_secs(1));
//! pin.set_low();
//! # Ok(())
//! # }
//! ```
//!
//! Additional examples can be found in the `examples` directory.
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! In recent releases of Raspberry Pi OS (December 2017 or later), users that are part of the
//! `gpio` group (like the default `pi` user) can access `/dev/gpiomem` and
//! `/dev/gpiochipN` (N = 0-2) without needing additional permissions. If you encounter any
//! [`PermissionDenied`] errors when constructing a new [`Gpio`] instance, either the current
//! user isn't a member of the `gpio` group, or your Raspberry Pi OS distribution isn't
//! up-to-date and doesn't automatically configure permissions for the above-mentioned
//! files. Updating Raspberry Pi OS to the latest release should fix any permission issues.
//! Alternatively, although not recommended, you can run your application with superuser
//! privileges by using `sudo`.
//!
//! If you're unable to update Raspberry Pi OS and its packages (namely `raspberrypi-sys-mods`) to
//! the latest available release, or updating hasn't fixed the issue, you might be able to
//! manually update your `udev` rules to set the appropriate permissions. More information
//! can be found at [raspberrypi/linux#1225] and [raspberrypi/linux#2289].
//!
//! [`Error::PinNotAvailable`]: enum.Error.html#variant.PinNotAvailable
//! [`PermissionDenied`]: enum.Error.html#variant.PermissionDenied
//! [raspberrypi/linux#1225]: https://github.com/raspberrypi/linux/issues/1225
//! [raspberrypi/linux#2289]: https://github.com/raspberrypi/linux/issues/2289
//! [`Gpio`]: struct.Gpio.html
//! [`Gpio::get`]: struct.Gpio.html#method.get
//! [`Pin`]: struct.Pin.html
//! [`InputPin`]: struct.InputPin.html
//! [`InputPin::set_reset_on_drop(false)`]: struct.InputPin.html#method.set_reset_on_drop
//! [`OutputPin`]: struct.OutputPin.html
//! [`OutputPin::set_reset_on_drop(false)`]: struct.OutputPin.html#method.set_reset_on_drop
#![allow(clippy::missing_transmute_annotations)]
#![allow(static_mut_refs)]

use std::fmt;
use std::io;
use std::ops::Not;

mod bcm;
mod pin;
mod system;

use bcm::GpioMem;
pub use pin::{InputPin, OutputPin, Pin};

/// Pin modes.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Input,
    Output,
    Alt0,
    Alt1,
    Alt2,
    Alt3,
    Alt4,
    Alt5,
    Alt6,
    Alt7,
    Alt8,
    Null,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Mode::Input => write!(f, "In"),
            Mode::Output => write!(f, "Out"),
            Mode::Alt0 => write!(f, "Alt0"),
            Mode::Alt1 => write!(f, "Alt1"),
            Mode::Alt2 => write!(f, "Alt2"),
            Mode::Alt3 => write!(f, "Alt3"),
            Mode::Alt4 => write!(f, "Alt4"),
            Mode::Alt5 => write!(f, "Alt5"),
            Mode::Alt6 => write!(f, "Alt6"),
            Mode::Alt7 => write!(f, "Alt7"),
            Mode::Alt8 => write!(f, "Alt8"),
            Mode::Null => write!(f, "Null"),
        }
    }
}

/// Pin logic levels.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Level {
    Low = 0,
    High = 1,
}

impl From<bool> for Level {
    fn from(e: bool) -> Level {
        if e { Level::High } else { Level::Low }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Level::Low => write!(f, "Low"),
            Level::High => write!(f, "High"),
        }
    }
}

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        if value == 0 { Level::Low } else { Level::High }
    }
}

impl Not for Level {
    type Output = Level;

    fn not(self) -> Level {
        match self {
            Level::Low => Level::High,
            Level::High => Level::Low,
        }
    }
}

/// Built-in pull-up/pull-down resistor states.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Bias {
    Off = 0b00,
    PullUp = 0b01,
    PullDown = 0b10,
}

impl fmt::Display for Bias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Bias::Off => write!(f, "Off"),
            Bias::PullUp => write!(f, "PullUp"),
            Bias::PullDown => write!(f, "PullDown"),
        }
    }
}

// Store Gpio's state separately, so we can conveniently share it through
// a cloned Arc.
#[derive(Debug)]
pub(crate) struct GpioState {
    gpio_mem: GpioMem,
}

/// Provides access to the Raspberry Pi's GPIO peripheral.
#[derive(Debug)]
pub struct Gpio {
    mem: GpioMem,
}

impl Gpio {
    /// Constructs a new `Gpio`.
    pub unsafe fn new() -> io::Result<Gpio> {
        Ok(Self { mem: GpioMem::open()? })
    }

    /// Returns a [`Pin`] for the specified BCM GPIO number.
    pub unsafe fn get(&self, pin: u8) -> Pin<'_> {
        unsafe { Pin::new(pin, &self.mem) }
    }
}
