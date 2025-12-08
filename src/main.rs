#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
mod devices;
mod gpib_command;
mod gpib_gpio;
mod gpib_pinout;
mod listener;
mod logger;
mod sniffer;
mod talker;
mod utils;

use std::{fs, io, path::Path};

use crate::{
    args::{Args, EmulatorArgs, SnifferArgs},
    devices::{DeviceManager, KnownDevice},
    logger::LogLevel,
    sniffer::BusSniffer,
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

    match args.command {
        args::Command::Emulator(args) => run_emulator(args),
        args::Command::Sniffer(args) => run_sniffer(args),
    }
}

fn run_emulator(args: EmulatorArgs) {
    let mut devman = DeviceManager::new();
    configure_devman(args, &mut devman);

    configure_scheduler();

    debug!("Configuration complete, start device emulator");
    devman.start();
}

fn configure_devman(args: EmulatorArgs, devman: &mut DeviceManager) {
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
        .expect(&format!("failed to open image {}", path.display()));

    // SAFETY: Maybe safe, I don't know.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) };

    return mmap.expect(&format!("failed to mmap image {}", path.display()));
}

fn run_sniffer(args: SnifferArgs) {
    let file = create_dump_file(&args.output_path, args.size).expect("failed to create dump file");
    let devman = BusSniffer::new(file);

    configure_scheduler();

    debug!("Configuration complete, start bus sniffer");
    devman.start();

    info!("Bus sniffer finished, dump saved to {}", args.output_path.display());
}

fn create_dump_file(path: &Path, size: usize) -> io::Result<memmap2::MmapMut> {
    if fs::exists(path)? {
        return Err(io::ErrorKind::AlreadyExists.into());
    }

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)?;

    file.set_len(size as u64)?;

    // SAFETY: Maybe safe, I don't know.
    unsafe { memmap2::MmapMut::map_mut(&file) }
}
