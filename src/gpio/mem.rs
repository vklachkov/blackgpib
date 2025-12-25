use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::ptr;

use libc::{self, MAP_FAILED, MAP_SHARED, O_SYNC, PROT_READ, PROT_WRITE, c_void, size_t};

use super::{
    pinout::KnownPin,
    types::{Level, PinMask, PinModesRegs},
};

const PATH_DEV_GPIOMEM: &str = "/dev/gpiomem";
// The BCM2835 has 41 32-bit registers related to the GPIO (datasheet @ 6.1).
// The BCM2711 (RPi4) has GPIO-related 32-bit registers #0 .. #60, an address space of 61 registers (datasheet @ 5.1).
const GPIO_MEM_REGISTERS: usize = 61;
const GPIO_MEM_SIZE: usize = GPIO_MEM_REGISTERS * std::mem::size_of::<u32>();

struct GpioReg(usize);

const GPFSEL0: GpioReg = GpioReg(0x00);
const GPFSEL1: GpioReg = GpioReg(0x01);
const GPFSEL2: GpioReg = GpioReg(0x02);
const GPSET0: GpioReg = GpioReg(0x1c / std::mem::size_of::<u32>());
const GPCLR0: GpioReg = GpioReg(0x28 / std::mem::size_of::<u32>());
const GPLEV0: GpioReg = GpioReg(0x34 / std::mem::size_of::<u32>());

#[derive(Debug)]
pub(super) struct GpioMem(*mut u32);

impl GpioMem {
    pub fn open() -> io::Result<GpioMem> {
        Self::map_devgpiomem().map(Self)
    }

    fn map_devgpiomem() -> io::Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist or /dev/gpiomem
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
    fn read(&self, reg: GpioReg) -> u32 {
        // SAFETY: Register addresses are valid.
        unsafe { ptr::read_volatile(self.0.add(reg.0)) }
    }

    #[inline(always)]
    fn write(&self, reg: GpioReg, value: u32) {
        // SAFETY: Register addresses are valid.
        unsafe { ptr::write_volatile(self.0.add(reg.0), value) }
    }
}

impl Drop for GpioMem {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.0 as *mut c_void, GPIO_MEM_SIZE as size_t) };
    }
}

impl GpioMem {
    #[inline(always)]
    pub fn set_high(&self, pin: KnownPin) {
        self.write(GPSET0, 1 << pin as usize);
    }

    #[inline(always)]
    pub fn set_pins_high(&self, mask: PinMask) {
        self.write(GPSET0, mask.value());
    }

    #[inline(always)]
    pub fn set_low(&self, pin: KnownPin) {
        self.write(GPCLR0, 1 << pin as usize);
    }

    #[inline(always)]
    pub fn levels(&self) -> u32 {
        return self.read(GPLEV0);
    }

    #[inline(always)]
    pub fn level(&self, pin: KnownPin) -> Level {
        let reg_value = self.read(GPLEV0);
        unsafe { std::mem::transmute((reg_value >> pin as usize) as u8 & 0b1) }
    }

    #[inline(always)]
    pub fn write_pins_modes(&self, regs: PinModesRegs) {
        let regs = regs.regs();

        self.write(GPFSEL0, regs[0]);
        self.write(GPFSEL1, regs[1]);
        self.write(GPFSEL2, regs[2]);
    }
}
