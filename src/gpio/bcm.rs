use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::time::Duration;

use libc::{self, MAP_FAILED, MAP_SHARED, O_SYNC, PROT_READ, PROT_WRITE, c_void, size_t};

use super::system::{DeviceInfo, SoC};
use super::{Bias, Level, Mode};

const PATH_DEV_GPIOMEM: &str = "/dev/gpiomem";
// The BCM2835 has 41 32-bit registers related to the GPIO (datasheet @ 6.1).
// The BCM2711 (RPi4) has GPIO-related 32-bit registers #0 .. #60, an address space of 61 registers (datasheet @ 5.1).
const GPIO_MEM_REGISTERS: usize = 61;
const GPIO_MEM_SIZE: usize = GPIO_MEM_REGISTERS * std::mem::size_of::<u32>();
const GPFSEL0: usize = 0x00;
const GPSET0: usize = 0x1c / std::mem::size_of::<u32>();
const GPCLR0: usize = 0x28 / std::mem::size_of::<u32>();
const GPLEV0: usize = 0x34 / std::mem::size_of::<u32>();
const GPPUD: usize = 0x94 / std::mem::size_of::<u32>();
const GPPUDCLK0: usize = 0x98 / std::mem::size_of::<u32>();
// Only available on BCM2711 (RPi4)
const GPPUD_CNTRL_REG0: usize = 0xe4 / std::mem::size_of::<u32>();

const FSEL_INPUT: u8 = 0b000;
const FSEL_OUTPUT: u8 = 0b001;
const FSEL_ALT0: u8 = 0b100;
const FSEL_ALT1: u8 = 0b101;
const FSEL_ALT2: u8 = 0b110;
const FSEL_ALT3: u8 = 0b111;
const FSEL_ALT4: u8 = 0b011;
const FSEL_ALT5: u8 = 0b010;

pub struct GpioMem {
    mem_ptr: *mut u32,
    soc: SoC,
}

impl fmt::Debug for GpioMem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpioMem")
            .field("mem_ptr", &self.mem_ptr)
            .field("locks", &format_args!("{{ .. }}"))
            .field("soc", &self.soc)
            .finish()
    }
}

impl GpioMem {
    pub fn open() -> io::Result<GpioMem> {
        // Try /dev/gpiomem first. If that fails, return error to user without trying /dev/mem instead.
        let mem_ptr = Self::map_devgpiomem()?;

        // Identify which SoC we're using.
        let soc = DeviceInfo::new()?.soc();

        Ok(GpioMem { mem_ptr, soc })
    }

    fn map_devgpiomem() -> io::Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist (< Raspbian Jessie), or /dev/gpiomem
        // doesn't have the appropriate permissions, or the current user is
        // not a member of the gpio group.
        let gpiomem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_SYNC)
            .open(PATH_DEV_GPIOMEM)?;

        // Memory-map /dev/gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(ptr::null_mut(), GPIO_MEM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, gpiomem_file.as_raw_fd(), 0)
        };

        if gpiomem_ptr == MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(gpiomem_ptr as *mut u32)
    }

    #[inline(always)]
    fn read(&self, offset: usize) -> u32 {
        unsafe { ptr::read_volatile(self.mem_ptr.add(offset)) }
    }

    #[inline(always)]
    fn write(&self, offset: usize, value: u32) {
        unsafe {
            ptr::write_volatile(self.mem_ptr.add(offset), value);
        }
    }
}

impl Drop for GpioMem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mem_ptr as *mut c_void, GPIO_MEM_SIZE as size_t);
        }
    }
}

impl GpioMem {
    #[inline(always)]
    pub fn set_high(&self, pin: u8) {
        let offset = GPSET0 + pin as usize / 32;
        let shift = pin % 32;

        self.write(offset, 1 << shift);
    }

    #[inline(always)]
    pub fn set_low(&self, pin: u8) {
        let offset = GPCLR0 + pin as usize / 32;
        let shift = pin % 32;

        self.write(offset, 1 << shift);
    }

    #[inline(always)]
    pub fn level(&self, pin: u8) -> Level {
        let offset = GPLEV0 + pin as usize / 32;
        let shift = pin % 32;
        let reg_value = self.read(offset);

        unsafe { std::mem::transmute((reg_value >> shift) as u8 & 0b1) }
    }

    pub fn mode(&self, pin: u8) -> Mode {
        let offset = GPFSEL0 + pin as usize / 10;
        let shift = (pin % 10) * 3;
        let reg_value = self.read(offset);

        match (reg_value >> shift) as u8 & 0b111 {
            FSEL_INPUT => Mode::Input,
            FSEL_OUTPUT => Mode::Output,
            FSEL_ALT0 => Mode::Alt0,
            FSEL_ALT1 => Mode::Alt1,
            FSEL_ALT2 => Mode::Alt2,
            FSEL_ALT3 => Mode::Alt3,
            FSEL_ALT4 => Mode::Alt4,
            FSEL_ALT5 => Mode::Alt5,
            _ => Mode::Input,
        }
    }

    pub fn set_mode(&self, pin: u8, mode: Mode) {
        let offset = GPFSEL0 + pin as usize / 10;
        let shift = (pin % 10) * 3;

        let fsel_mode = match mode {
            Mode::Input => FSEL_INPUT,
            Mode::Output => FSEL_OUTPUT,
            Mode::Alt0 => FSEL_ALT0,
            Mode::Alt1 => FSEL_ALT1,
            Mode::Alt2 => FSEL_ALT2,
            Mode::Alt3 => FSEL_ALT3,
            Mode::Alt4 => FSEL_ALT4,
            Mode::Alt5 => FSEL_ALT5,
            _ => FSEL_INPUT,
        };

        let reg_value = self.read(offset);
        self.write(offset, (reg_value & !(0b111 << shift)) | ((fsel_mode as u32) << shift));
    }

    pub fn set_bias(&self, pin: u8, bias: Bias) {
        // Offset for register.
        let offset: usize;
        // Bit shift for pin position within register value.
        let shift: u8;

        // BCM2711 (RPi4) and BCM2712 (RPi5) need special handling.
        if self.soc == SoC::Bcm2711 || self.soc == SoC::Bcm2712 {
            offset = GPPUD_CNTRL_REG0 + pin as usize / 16;
            shift = pin % 16 * 2;

            // Pull up vs pull down has a reverse bit pattern on BCM2711 vs others.
            let pud = bias as u32;
            let reg_value = self.read(offset);
            self.write(offset, (reg_value & !(0b11 << shift)) | (pud << shift));
        } else {
            offset = GPPUDCLK0 + pin as usize / 32;
            shift = pin % 32;

            // Set the control signal in GPPUD.
            let reg_value = self.read(GPPUD);
            self.write(GPPUD, (reg_value & !0b11) | ((bias as u32) & 0b11));

            // The datasheet mentions waiting at least 150 cycles for set-up and hold, but
            // doesn't state which clock is used. This is likely the VPU clock (see
            // https://www.raspberrypi.org/forums/viewtopic.php?f=72&t=163352). At either
            // 250MHz or 400MHz, a 5µs delay + overhead is more than adequate.

            // Set-up time for the control signal. >= 600ns
            crate::utils::busy_wait(800);
            // Clock the control signal into the selected pin.
            self.write(offset, 1 << shift);

            // Hold time for the control signal. >= 600ns
            crate::utils::busy_wait(800);
            // Remove the control signal and clock.
            self.write(GPPUD, reg_value & !0b11);
            self.write(offset, 0);
        }
    }
}
