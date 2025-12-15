#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
mod controller;
mod emulator;
mod gpib_command;
mod gpib_gpio;
mod gpib_pinout;
mod listener;
mod logger;
mod sniffer;
mod talker;
mod utils;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    args::{Args, ControllerCommand, EmulatorArgs, SnifferArgs},
    controller::DeviceController,
    emulator::DeviceEmulator,
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

    configure_scheduler();

    match args.command {
        args::Command::Emulator(args) => run_emulator(args),
        args::Command::Sniffer(args) => run_sniffer(args),
        args::Command::Controller(cmd) => run_controller(cmd).expect("controller panicked"),
    }
}

fn run_emulator(args: EmulatorArgs) {
    let mut emulator = DeviceEmulator::new();
    configure_emulator(args, &mut emulator);

    debug!("Configuration complete, start device emulator");
    emulator.start();
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

fn run_controller(cmd: ControllerCommand) -> io::Result<()> {
    match cmd {
        ControllerCommand::Format { address, validate } => {
            DeviceController::new_with_reset(address).format_disk(validate)
        }
        ControllerCommand::Write {
            from_path: path,
            to_address: address,
        } => {
            let mut controller = DeviceController::new_with_reset(address);

            let disk_status = controller.read_status()?;
            let image_size = disk_status.sector_size as usize * disk_status.sector_count as usize;

            let file = open_disk_copy_file(path, image_size)?;
            controller.write_image_to_disk(file)
        }
        ControllerCommand::Read {
            from_address: address,
            to_path: path,
        } => {
            let mut controller = DeviceController::new_with_reset(address);

            let disk_status = controller.read_status()?;
            let image_size = disk_status.sector_size as usize * disk_status.sector_count as usize;

            let file = open_image_file(path, image_size)?;
            controller.read_disk_to_writer(file)
        }
    }
}

fn open_disk_copy_file(path: PathBuf, image_size: usize) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(false)
        .write(true)
        .open(path)?;

    file.lock()?;

    file.set_len(image_size as u64)?;

    Ok(file)
}

fn open_image_file(path: PathBuf, image_size: usize) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(false)
        .read(true)
        .write(false)
        .open(path)?;

    file.lock()?;

    let file_len = file.metadata()?.len();
    if file_len != image_size as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "TODO"));
    }

    Ok(file)
}
