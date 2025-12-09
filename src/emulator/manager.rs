use memmap2::MmapMut;

use crate::{debug, error, gpib_command::GPIBCommand, listener::Listener, talker::Talker, trace, warn};

use super::{
    device::{Device, ServiceRequest},
    disk::Disk,
    proxy::DataToSocketDevice,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SerialPollState {
    /// There was no Service Request yet.
    Init,

    /// A Service Request was requested by device.
    Requested(u8),

    /// The laptop sent SPE. The next Talk will be interpreted as an attempt
    /// to find which device made the Service Request.
    Enabled(u8),

    /// The laptop sent SPD. However, after SPD, SPE can still be sent again,
    /// so we need to remember the device.
    Disabled(u8),
}

impl SerialPollState {
    fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled(_))
    }

    fn to_enabled(self) -> Self {
        match self {
            Self::Requested(d) => Self::Enabled(d),
            Self::Disabled(d) => Self::Enabled(d),
            _ => self,
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
    /// This request is not for the device itself, the laptop is just checking
    /// which device made the Service Request.
    ///
    /// The parameter shows if this device made the request.
    SerialPollProbe(bool),

    /// The laptop told the device to talk.
    Device(&'a mut dyn Device),
}

pub struct DeviceEmulator {
    devices: Box<[Option<Box<dyn Device>>; Self::MAX_GPIB_DEVICES]>,
    active_listener: Option<u8>,
    serial_poll_state: SerialPollState,
}

impl DeviceEmulator {
    const MAX_GPIB_DEVICES: usize = 31;

    pub fn new() -> Self {
        Self {
            devices: Box::default(),
            active_listener: None,
            serial_poll_state: SerialPollState::Init,
        }
    }

    pub fn create_disk(&mut self, address: u8, image: MmapMut) {
        self.new_device(address, || {
            let name = format!("Disk {address:#04x}");
            Disk::new(name, image)
        });
    }

    pub fn create_proxy(&mut self, address: u8, port: u16) {
        self.new_device(address, || DataToSocketDevice::new(port));
    }

    fn new_device<T, F>(&mut self, address: u8, ctor: F)
    where
        T: Device + 'static,
        F: FnOnce() -> T,
    {
        let id = address as usize;

        assert!(id < Self::MAX_GPIB_DEVICES, "address must be in range 0..=30");
        assert!(self.devices[id].is_none(), "device with address {id} already exists");

        self.devices[id] = Some(Box::new(ctor()))
    }

    pub fn start(mut self) {
        loop {
            let mut listener = Listener::new(false);

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
                        if let Some(d) = self.active_listener {
                            self.get_device(d).reset()
                        }
                    }
                    GPIBCommand::SPE => {
                        self.serial_poll_state = self.serial_poll_state.to_enabled();
                    }
                    GPIBCommand::SPD => {
                        self.serial_poll_state = self.serial_poll_state.to_disabled();
                    }
                    GPIBCommand::MLA(address) => {
                        if self.is_device_exists(address) {
                            self.active_listener = Some(address);
                        } else {
                            listener.wait_next_command();
                        }
                    }
                    GPIBCommand::UNL => {
                        self.process_unlisten(&mut listener);
                    }
                    GPIBCommand::MTA(address) => {
                        if !self.is_device_exists(address) {
                            listener.wait_next_command();
                            continue;
                        }

                        if self.serial_poll_state.is_enabled() {
                            let is_requester = self.serial_poll_state == SerialPollState::Enabled(address);
                            break 'l TalkMode::SerialPollProbe(is_requester);
                        } else {
                            break 'l TalkMode::Device(self.get_device(address));
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
    fn is_device_exists(&self, address: u8) -> bool {
        match self.devices.get(address as usize) {
            Some(device) => device.is_some(),
            None => false,
        }
    }

    #[inline]
    fn get_device(&mut self, address: u8) -> &mut dyn Device {
        self.devices
            .get_mut(address as usize)
            .expect("address must be in range 0..=30")
            .as_mut()
            .expect("device must exists")
            .as_mut()
    }

    fn process_byte(&mut self, listener: &mut Listener, byte: u8, eoi: bool) {
        let Some(active_listener) = self.active_listener else {
            return self.atn_broken(listener, byte);
        };

        self.get_device(active_listener).process_byte(byte, eoi);
    }

    fn atn_broken(&mut self, listener: &mut Listener, byte: u8) {
        let command = GPIBCommand::from(byte);
        error!(
            "Laptop sent byte {byte:#04x} (maybe {command:?} command) to the bus without MLA command. Probably, GPiB state is broken"
        );

        warn!("Reset all devices and wait for next command...");

        self.reset_all();

        listener.wait_next_command();
    }

    fn process_unlisten(&mut self, listener: &mut Listener) {
        let Some(active_listener) = self.active_listener.take() else {
            warn!("Laptop sent UNL to the bus without MLA command");
            return;
        };

        let service_request = self.get_device(active_listener).unlisten();
        if service_request == ServiceRequest::Required {
            self.serial_poll_state = SerialPollState::Requested(active_listener);
            listener.service_request();
        }
    }

    fn reset_all(&mut self) {
        self.active_listener = None;
        self.serial_poll_state = SerialPollState::Init;

        for device in self.devices.iter_mut() {
            if let Some(device) = device {
                device.reset();
            }
        }
    }
}
