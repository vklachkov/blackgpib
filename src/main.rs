#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
mod controller;
mod emulator;
mod gpib_command;
mod gpio;
mod listener;
mod logger;
mod sniffer;
mod system;
mod talker;
mod utils;

use std::{fs, io, path::Path};

use crate::{
    args::{Args, ControllerArgs, EmulatorArgs, SnifferArgs},
    controller::DeviceController,
    emulator::DeviceEmulator,
    gpio::Gpio,
    logger::LogLevel,
    sniffer::BusSniffer,
    system::{DeviceInfo, GpioInterface},
    utils::configure_scheduler,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = Args::parse();

    setup_logger(&args);

    let gpio = open_gpio().expect("Failed to configure gpio");

    match args.command {
        args::Command::Emulator(args) => run_emulator(args, gpio),
        args::Command::Sniffer(args) => run_sniffer(args),
        args::Command::Controller(args) => run_controller(args),
    }
}

fn setup_logger(args: &Args) {
    logger::setup(if args.trace {
        LogLevel::Trace
    } else if args.verbose {
        LogLevel::Debug
    } else {
        LogLevel::Info
    });

    info!("BlackGPiB v{VERSION} started");
}

fn open_gpio() -> io::Result<Gpio> {
    let device_info = DeviceInfo::new()?;
    if device_info.gpio_interface() == GpioInterface::Rp1 {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "RP1 does not supported"));
    }

    // SAFETY: TODO.
    unsafe { Gpio::new(&device_info) }
}

fn run_emulator(args: EmulatorArgs, gpio: Gpio) {
    let mut emulator = DeviceEmulator::new();
    configure_emulator(args, &mut emulator);

    configure_scheduler();

    debug!("Configuration complete, start device emulator");
    emulator.start(gpio);
}

fn configure_emulator(args: EmulatorArgs, emulator: &mut DeviceEmulator) {
    emulator.create_proxy(21, 49274); // default printer
    emulator.create_proxy(25, 49275); // printer hp
    emulator.create_proxy(20, 49276); // plotter

    if let Some(ref path) = args.hdd_1_image {
        emulator.create_disk(04, mmap_disk_image(path));
    }
    if let Some(ref path) = args.floppy_drive_1_image {
        emulator.create_disk(05, mmap_disk_image(path));
    }
    if let Some(ref path) = args.portable_floppy_image {
        emulator.create_disk(06, mmap_disk_image(path));
    }
    if let Some(ref path) = args.hdd_2_image {
        emulator.create_disk(12, mmap_disk_image(path));
    }
    if let Some(ref path) = args.floppy_drive_2_image {
        emulator.create_disk(13, mmap_disk_image(path));
    }
}

fn mmap_disk_image(path: &Path) -> memmap2::MmapMut {
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
    let sniffer = BusSniffer::new(file);

    configure_scheduler();

    debug!("Configuration complete, start bus sniffer");
    sniffer.start();

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

fn run_controller(args: ControllerArgs) {
    let controller = DeviceController::new(args.address);

    configure_scheduler();

    debug!("Configuration complete");
    controller.start();
}
