#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod devices;
mod gpib_command;
mod gpib_gpio;
mod gpib_pinout;
mod listener;
mod logger;
mod talker;
mod utils;

use std::fs;

use crate::{
    devices::{DeviceManager, KnownDevice},
    logger::LogLevel,
    utils::configure_scheduler,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    logger::setup(LogLevel::Info);

    info!("BlackGPiB v{VERSION} started");

    debug!("Reset all pins to Z-State...");
    gpib_gpio::reset_all();

    debug!("Setup scheduler...");
    configure_scheduler();

    let mut devman = DeviceManager::new();

    // FIXME: Remove hardcoded images.
    devman.insert_image(KnownDevice::HardDisk, fs::read("XECUT_BOOT.IMG").unwrap(), 0x121, 0x120);
    devman.insert_image(KnownDevice::FloppyDrive, fs::read("disk1").unwrap(), 0x121, 0x120);
    devman.insert_image(KnownDevice::PortableFloppy, fs::read("2101_6ext_fixed").unwrap(), 0x121, 0x120);
    devman.insert_image(KnownDevice::HardDisk2, fs::read("GRIDOS.IMG").unwrap(), 0x121, 0x120);
    devman.insert_image(KnownDevice::FloppyDrive2, fs::read("disk1").unwrap(), 0x121, 0x120);

    devman.start();
}
