use super::pinout::KnownPin;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Input = 0b000,
    Output = 0b001,
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

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        if value == 0 { Level::Low } else { Level::High }
    }
}

impl std::ops::Not for Level {
    type Output = Level;

    fn not(self) -> Level {
        match self {
            Level::Low => Level::High,
            Level::High => Level::Low,
        }
    }
}

pub struct PinModesRegs {
    values: [u32; 3],
}

impl PinModesRegs {
    pub const fn new() -> Self {
        Self { values: [0; 3] }
    }

    pub const fn set(&mut self, pin: KnownPin, mode: Mode) {
        let pin = pin as usize;

        let reg = pin / 10;
        let offset = (pin % 10) * 3;

        self.values[reg] |= (mode as u32) << offset;
    }

    pub const fn regs(&self) -> [u32; 3] {
        self.values
    }
}

pub struct PinMask {
    mask: u32,
}

impl PinMask {
    pub const fn new() -> Self {
        Self { mask: 0 }
    }

    pub const fn set(&mut self, pin: KnownPin) {
        self.mask |= 1 << (pin as usize);
    }

    pub const fn value(&self) -> u32 {
        self.mask
    }
}
