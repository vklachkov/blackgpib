mod device;
mod disk;
mod proxy;

use std::time::Instant;

use crate::{
    common::{CommonPins, reset_all_pins},
    debug,
    gpib_command::GPIBCommand,
    listener::Listener,
    rppal::Gpio,
    talker::Talker,
    warn,
};

use device::{Device, ServiceRequest};
use disk::Disk;
use proxy::DataToSocketDevice;

use memmap2::MmapMut;

const MAX_GPIB_DEVICES: usize = 31;

type Devices = [Option<Box<dyn Device>>; MAX_GPIB_DEVICES];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SerialPollState {
    /// There was no Service Request yet.
    Disabled,

    /// A Service Request was requested by device.
    Requested(u8),

    /// The laptop sent SPE. The next Talk will be interpreted as an attempt
    /// to find which device made the Service Request.
    UnexpectedSPE,
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
    devices: Box<Devices>,
    active_listener: Option<u8>,
    active_talker: Option<u8>,
    serial_poll_state: SerialPollState,
    listen_buffer: Vec<u8>,
}

impl DeviceEmulator {
    pub fn new() -> Self {
        Self {
            devices: Box::default(),
            active_listener: None,
            active_talker: None,
            serial_poll_state: SerialPollState::Disabled,
            listen_buffer: Vec::with_capacity(10 * 1024),
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

        assert!(id < MAX_GPIB_DEVICES, "address must be in range 0..=30");
        assert!(self.devices[id].is_none(), "device with address {id} already exists");

        self.devices[id] = Some(Box::new(ctor()))
    }

    pub fn start(mut self) {
        let gpio = unsafe { Gpio::new() }.unwrap();
        reset_all_pins(&gpio);

        let common_pins = CommonPins::new(&gpio);

        loop {
            let a = Instant::now();
            let listener = Listener::new(&gpio, &common_pins);
            crate::info!("Listener setup {:?}", a.elapsed());

            let talk_mode = 'l: loop {
                let cmd = listener.start_command_handshake();
                crate::info!("Accept command {cmd:?}");

                match *cmd {
                    GPIBCommand::DCL => {
                        self.reset_all();
                    }
                    GPIBCommand::SPE => match self.serial_poll_state {
                        SerialPollState::Disabled => {
                            self.serial_poll_state = SerialPollState::UnexpectedSPE;
                        }
                        SerialPollState::Requested(_) => {
                            cmd.expected();
                        }
                        SerialPollState::UnexpectedSPE => {
                            cmd.unexpected();
                        }
                    },
                    GPIBCommand::SPD => {
                        match self.serial_poll_state {
                            SerialPollState::Disabled => cmd.unexpected(),
                            SerialPollState::Requested(_) => cmd.expected(),
                            SerialPollState::UnexpectedSPE => cmd.unexpected(),
                        }

                        self.serial_poll_state = SerialPollState::Disabled;
                    }
                    GPIBCommand::MLA(address) => {
                        if self.is_device_exists(address) {
                            cmd.expected();
                            crate::trace!("MLA, listen to buffer");
                            self.listen_to_buffer(&listener, address);
                        } else {
                            cmd.unexpected();
                        }
                    }
                    GPIBCommand::UNL => {
                        if let Some(active_listener) = self.active_listener.take() {
                            cmd.expected();
                            self.process_bytes(&listener, active_listener);
                        } else {
                            cmd.unexpected();
                        }
                    }
                    GPIBCommand::MTA(address) => {
                        if self.is_device_exists(address) {
                            self.active_talker = Some(address);

                            if self.serial_poll_state == SerialPollState::Disabled {
                                break 'l TalkMode::Device(Self::get_device(&mut self.devices, address));
                            } else {
                                let f = self.serial_poll_state == SerialPollState::Requested(address);
                                break 'l TalkMode::SerialPollProbe(f);
                            }
                        } else {
                            cmd.unexpected();
                        }
                    }
                    GPIBCommand::UNT => {
                        if self.active_talker.take().is_some() {
                            cmd.expected();
                        } else {
                            cmd.unexpected();
                        }
                    }
                    GPIBCommand::Unsupported(value) => {
                        warn!("Unsupported command {value:#04x}");
                        cmd.unexpected();
                    }
                }
            };

            drop(listener);

            let talker = Talker::new(&gpio, &common_pins);

            match talk_mode {
                TalkMode::SerialPollProbe(valid_address) => {
                    let response = if valid_address { 0x4F } else { 0x0F };
                    talker.send_serial_poll_response(response);
                }
                TalkMode::Device(device) => {
                    device.talk(talker);
                }
            }
        }
    }

    fn reset_all(&mut self) {
        self.active_listener = None;
        self.serial_poll_state = SerialPollState::Disabled;
        self.listen_buffer.clear();

        for device in self.devices.iter_mut() {
            if let Some(device) = device {
                device.reset();
            }
        }
    }

    fn is_device_exists(&self, address: u8) -> bool {
        match self.devices.get(address as usize) {
            Some(device) => device.is_some(),
            None => false,
        }
    }

    fn get_device(devices: &mut Devices, address: u8) -> &mut dyn Device {
        devices
            .get_mut(address as usize)
            .expect("address must be in range 0..=30")
            .as_mut()
            .expect("device must exists")
            .as_mut()
    }

    fn listen_to_buffer(&mut self, listener: &Listener, address: u8) {
        self.active_listener = Some(address);
        self.listen_buffer.clear();

        loop {
            let byte = *listener.start_data_handshake();

            if byte.atn {
                self.reset_active_listener();
                break;
            }

            self.listen_buffer.push(byte.value);

            if byte.eoi {
                break;
            }
        }

        debug!("in buffer {} bytes", self.listen_buffer.len());
    }

    fn reset_active_listener(&mut self) {
        let Some(active_listener) = self.active_listener else {
            return;
        };

        self.active_listener = None;
        self.listen_buffer.clear();

        Self::get_device(&mut self.devices, active_listener).reset();
    }

    fn process_bytes(&mut self, listener: &Listener, address: u8) {
        let device = Self::get_device(&mut self.devices, address);

        let service_request = device.process_bytes(&self.listen_buffer);
        if service_request == ServiceRequest::Required {
            self.serial_poll_state = SerialPollState::Requested(address);
            listener.service_request();
        }

        self.active_listener = None;
        self.listen_buffer.clear();
    }
}
