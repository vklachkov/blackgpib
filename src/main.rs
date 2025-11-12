#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
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
    args::Args,
    devices::{DeviceManager, KnownDevice},
    logger::LogLevel,
    utils::configure_scheduler,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = Args::parse();

    logger::setup(LogLevel::Info);

    info!("BlackGPiB v{VERSION} started");

    debug!("Reset all pins to Z-State...");
    gpib_gpio::reset_all();

    debug!("Setup scheduler...");
    configure_scheduler();

    let mut devman = DeviceManager::new();
    configure_devman(args, &mut devman);

    debug!("Configuration complete, device manager started");
    devman.start();
}

fn configure_devman(args: Args, devman: &mut DeviceManager) {
    if let Some(ref path) = args.hdd_1_image {
        let image = fs::read(path).expect("Failed to read HDD 1 image");
        devman.insert_image(KnownDevice::HardDisk, image, 0x121, 0x120);
    }
    if let Some(ref path) = args.floppy_drive_1_image {
        let image = fs::read(path).expect("Failed to read Floppy Drive 1 image");
        devman.insert_image(KnownDevice::FloppyDrive, image, 0x121, 0x120);
    }
    if let Some(ref path) = args.portable_floppy_image {
        let image = fs::read(path).expect("Failed to read Portable Floppy image");
        devman.insert_image(KnownDevice::PortableFloppy, image, 0x121, 0x120);
    }
    if let Some(ref path) = args.hdd_2_image {
        let image = fs::read(path).expect("Failed to read HDD 2 image");
        devman.insert_image(KnownDevice::HardDisk2, image, 0x121, 0x120);
    }
    if let Some(ref path) = args.floppy_drive_2_image {
        let image = fs::read(path).expect("Failed to read Floppy Drive 2 image");
        devman.insert_image(KnownDevice::FloppyDrive2, image, 0x121, 0x120);
    }
}
