mod device;
mod disk;
mod manager;
mod printer;

pub use manager::DeviceManager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownDevice {
    HardDisk,
    FloppyDrive,
    PortableFloppy,
    HardDisk2,
    FloppyDrive2,
    Printer,
}

impl KnownDevice {
    pub(crate) fn from_address(address: u8) -> Option<Self> {
        match address {
            4 => Some(Self::HardDisk),
            5 => Some(Self::FloppyDrive),
            6 => Some(Self::PortableFloppy),
            12 => Some(Self::HardDisk2),
            13 => Some(Self::FloppyDrive2),
            25 => Some(Self::Printer),
            _ => None,
        }
    }
}
