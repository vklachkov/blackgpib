use crate::{debug, error, gpib_command::GPIBCommand, listener::Listener, talker::Talker, trace, warn};

use super::{Device, disk::Disk, printer::GenericPrinter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnownDevice {
    HardDisk,
    FloppyDrive,
    PortableFloppy,
    HardDisk2,
    FloppyDrive2,
    Printer,
}

impl KnownDevice {
    fn from_address(address: u8) -> Option<Self> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SerialPollState {
    Init,
    Requested(KnownDevice),
    Enabled(KnownDevice),
    Disabled(KnownDevice),
}

impl SerialPollState {
    fn to_enabled(self) -> Self {
        match self {
            Self::Requested(d) => Self::Enabled(d),
            Self::Disabled(d) => Self::Enabled(d),
            _ => self,
        }
    }

    fn is_enabled(self) -> bool {
        match self {
            Self::Enabled(_) => true,
            _ => false,
        }
    }

    fn to_disabled(self) -> Self {
        match self {
            Self::Enabled(d) => Self::Disabled(d),
            _ => self,
        }
    }
}

enum TalkMode<'a> {
    SerialPollProbe(bool),
    Device(&'a mut dyn Device),
}

pub struct DeviceManager {
    disks: [Disk; 5],
    printer: GenericPrinter,
    active_listener: Option<KnownDevice>,
    serial_poll_state: SerialPollState,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            disks: [
                Disk::new("Disk 0x04"),
                Disk::new("Disk 0x05"),
                Disk::new("Disk 0x06"),
                Disk::new("Disk 0x0C"),
                Disk::new("Disk 0x0D"),
            ],
            printer: GenericPrinter::new(),
            active_listener: None,
            serial_poll_state: SerialPollState::Init,
        }
    }

    pub fn insert_image(&mut self, disk: u8, image: Vec<u8>, superblock_id: u16, bitmap_block_id: u16) {
        self.disks[disk as usize].use_image(image, superblock_id, bitmap_block_id);
    }

    pub fn start(mut self) {
        loop {
            let mut listener = Listener::new();

            let talk_mode = 'l: loop {
                let byte = listener.handshake_byte();
                trace!(
                    "Accept byte {:#010b} ({:#04x}) ATN={} EOI={}",
                    byte.value, byte.value, byte.atn as u8, byte.eoi as u8
                );

                if !byte.atn {
                    self.process_byte(&mut listener, byte.value, byte.eoi);
                    continue;
                }

                let cmd = GPIBCommand::from(byte.value);
                debug!("Accept command {cmd:?}");

                match cmd {
                    GPIBCommand::DCL => {
                        self.reset_all();
                    }
                    GPIBCommand::SDC => {
                        self.active_listener.map(|d| self.get_device(d).reset());
                    }
                    GPIBCommand::SPE => {
                        self.serial_poll_state = self.serial_poll_state.to_enabled();
                    }
                    GPIBCommand::SPD => {
                        self.serial_poll_state = self.serial_poll_state.to_disabled();
                    }
                    GPIBCommand::MLA(address) => {
                        let Some(device) = KnownDevice::from_address(address) else {
                            listener.wait_next_command();
                            continue;
                        };

                        self.active_listener = Some(device);
                    }
                    GPIBCommand::UNL => {
                        self.active_listener = None;
                    }
                    GPIBCommand::MTA(address) => {
                        let Some(talk_device) = KnownDevice::from_address(address) else {
                            listener.wait_next_command();
                            continue;
                        };

                        if self.serial_poll_state.is_enabled() {
                            let is_requester = self.serial_poll_state == SerialPollState::Enabled(talk_device);
                            break 'l TalkMode::SerialPollProbe(is_requester);
                        } else {
                            break 'l TalkMode::Device(self.get_device(talk_device));
                        }
                    }
                    GPIBCommand::UNT => {
                        continue;
                    }
                    GPIBCommand::Unsupported(cmd) => {
                        warn!("Unsupported command {cmd:#04x}");
                    }
                }
            };

            drop(listener);

            let mut talker = Talker::new();

            match talk_mode {
                TalkMode::SerialPollProbe(valid_address) => {
                    let response = if valid_address { 0x4F } else { 0x0F };
                    talker.send_byte(response, false, false);
                }
                TalkMode::Device(device) => {
                    device.talk(talker);
                }
            }
        }
    }

    #[inline]
    fn get_device(&mut self, device: KnownDevice) -> &mut dyn Device {
        match device {
            KnownDevice::HardDisk => &mut self.disks[0],
            KnownDevice::FloppyDrive => &mut self.disks[1],
            KnownDevice::PortableFloppy => &mut self.disks[2],
            KnownDevice::HardDisk2 => &mut self.disks[3],
            KnownDevice::FloppyDrive2 => &mut self.disks[4],
            KnownDevice::Printer => &mut self.printer,
        }
    }

    fn process_byte(&mut self, listener: &mut Listener, byte: u8, eoi: bool) {
        let Some(active_listener) = self.active_listener else {
            error!("The laptop sent byte {byte:#010b} to the bus without an MLA command");
            return;
        };

        let device = self.get_device(active_listener);

        let service_request = device.process_byte(byte, eoi);
        if service_request {
            self.serial_poll_state = SerialPollState::Requested(active_listener);
            listener.service_request();
        }
    }

    fn reset_all(&mut self) {
        self.active_listener = None;
        self.serial_poll_state = SerialPollState::Init;

        for disk in &mut self.disks {
            disk.reset();
        }

        self.printer.reset();
    }
}
