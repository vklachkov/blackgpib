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

use std::{fs, path::Path};

use crate::{
    args::Args,
    devices::{DeviceManager, KnownDevice},
    logger::LogLevel,
    utils::configure_scheduler,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = Args::parse();

    logger::setup(if args.trace {
        LogLevel::Trace
    } else if args.verbose {
        LogLevel::Debug
    } else {
        LogLevel::Info
    });

    info!("BlackGPiB v{VERSION} started");

    debug!("Reset all pins to Z-State...");
    gpib_gpio::reset_all();

    let mut devman = DeviceManager::new();
    configure_devman(args, &mut devman);

    debug!("Setup scheduler...");
    configure_scheduler();

    debug!("Configuration complete, device manager started");
    devman.start();
}

fn configure_devman(args: Args, devman: &mut DeviceManager) {
    if let Some(ref path) = args.hdd_1_image {
        devman.insert_image(KnownDevice::HardDisk, mmap_file(path), 0x121, 0x120);
    }
    if let Some(ref path) = args.floppy_drive_1_image {
        devman.insert_image(KnownDevice::FloppyDrive, mmap_file(path), 0x121, 0x120);
    }
    if let Some(ref path) = args.portable_floppy_image {
        devman.insert_image(KnownDevice::PortableFloppy, mmap_file(path), 0x121, 0x120);
    }
    if let Some(ref path) = args.hdd_2_image {
        devman.insert_image(KnownDevice::HardDisk2, mmap_file(path), 0x121, 0x120);
    }
    if let Some(ref path) = args.floppy_drive_2_image {
        devman.insert_image(KnownDevice::FloppyDrive2, mmap_file(path), 0x121, 0x120);
    }
}

fn mmap_file(path: &Path) -> memmap2::MmapMut {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .truncate(false)
        .open(path)
        .expect(&format!("Failed to open image {}", path.display()));

    // SAFETY: Maybe safe, I don't know.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) };

    return mmap.expect(&format!("Failed to mmap image {}", path.display()));
}
