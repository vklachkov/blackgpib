#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
mod controller;
mod disk_protocol;
mod emulator;
mod gpib;
mod gpio;
mod logger;
mod sniffer;
mod system;
mod time_utils;

use std::{fs, io, path::Path, process::ExitCode, time::Instant};

use crate::{
    args::{Args, ControllerCommand, EmulatorArgs, SnifferArgs},
    controller::DeviceController,
    disk_protocol::DiskIdentity,
    emulator::DeviceEmulator,
    gpio::Gpio,
    logger::LogLevel,
    sniffer::BusSniffer,
    system::{DeviceInfo, GpioInterface, SoC},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args = Args::parse();

    setup_logger(&args);

    match blackgpib(args) {
        Ok(()) => {
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            error!("BlackGPiB fatal error: {err}");
            return ExitCode::FAILURE;
        }
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
}

fn blackgpib(args: Args) -> io::Result<()> {
    check_device_compatibility()?;
    let gpio = open_gpio()?;

    configure_scheduler()?;

    match args.command {
        args::Command::Emulator(args) => run_emulator(gpio, args),
        args::Command::Sniffer(args) => run_sniffer(gpio, args),
        args::Command::Controller(cmd) => run_controller(gpio, cmd),
    }
}

fn check_device_compatibility() -> io::Result<()> {
    trace!("Check device compatibility");

    match DeviceInfo::new() {
        Ok(info) => match info.gpio_interface() {
            GpioInterface::Bcm if info.soc() != SoC::Bcm2835 => {
                info!("Detected supported {} ({})", info.model(), info.soc());
                return Ok(());
            }
            _ => {
                error!("Sorry, your {} ({}) does not supported :(", info.model(), info.soc());
                return Err(io::Error::new(io::ErrorKind::Unsupported, "detected unsupported Raspberry Pi"));
            }
        },
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "unknown Raspberry Pi model"));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

fn open_gpio() -> io::Result<Gpio> {
    trace!("Open GPIO");

    // SAFETY: Initialized only once; no other GPIO exist.
    unsafe { Gpio::new() }
}

fn configure_scheduler() -> io::Result<()> {
    trace!("Pin blackgpib to core 3 and set priority");

    // SAFETY: CPU affinity mask is configured per the documentation.
    let cpu_mask = unsafe {
        let mut set = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(3, &mut set);
        set
    };

    // SAFETY: This call does not cause UB; on error it returns -1.
    let result = unsafe { libc::sched_setaffinity(0, size_of_val(&cpu_mask), &cpu_mask) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: This call does not cause UB; on error it returns -1.
    // let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -19) };
    // if result == -1 {
    //     return Err(io::Error::last_os_error());
    // }

    Ok(())
}

fn run_emulator(gpio: Gpio, args: EmulatorArgs) -> io::Result<()> {
    let mut emulator = DeviceEmulator::new();

    debug!("Configure emulator before start");
    configure_emulator(args, &mut emulator)?;

    info!("Start BlackGPiB v{VERSION} emulator");
    emulator.start(gpio);

    Ok(())
}

fn configure_emulator(args: EmulatorArgs, emulator: &mut DeviceEmulator) -> io::Result<()> {
    emulator.create_proxy(21, 49274)?; // default printer
    emulator.create_proxy(25, 49275)?; // printer hp
    emulator.create_proxy(20, 49276)?; // plotter

    if let Some(ref path) = args.hdd_1_image {
        emulator.create_disk(04, mmap_disk_image(path)?)?;
    }
    if let Some(ref path) = args.floppy_drive_1_image {
        emulator.create_disk(05, mmap_disk_image(path)?)?;
    }
    if let Some(ref path) = args.portable_floppy_image {
        emulator.create_disk(06, mmap_disk_image(path)?)?;
    }
    if let Some(ref path) = args.hdd_2_image {
        emulator.create_disk(12, mmap_disk_image(path)?)?;
    }
    if let Some(ref path) = args.floppy_drive_2_image {
        emulator.create_disk(13, mmap_disk_image(path)?)?;
    }

    Ok(())
}

fn mmap_disk_image(path: &Path) -> io::Result<memmap2::MmapMut> {
    debug!("Open disk image {}", path.display());

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .truncate(false)
        .open(path)
        .map_err(|err| map_file_error(err, path, "open"))?;

    file.lock().map_err(|err| map_file_error(err, path, "lock"))?;

    // SAFETY: The file is opened and locked; no other process can access it.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) }.map_err(|err| map_file_error(err, path, "mmap"))?;

    // The file is needed for the entire emulator lifetime; it must not be dropped.
    std::mem::forget(file);

    return Ok(mmap);
}

fn run_sniffer(gpio: Gpio, args: SnifferArgs) -> io::Result<()> {
    let file = create_dump_file(&args.output_path, args.size)?;
    let sniffer = BusSniffer::new(file);

    info!("Start BlackGPiB v{VERSION} sniffer");

    sniffer.start(gpio);

    info!("Bus sniffer finished, dump saved to {}", args.output_path.display());

    Ok(())
}

fn create_dump_file(path: &Path, size: usize) -> io::Result<memmap2::MmapMut> {
    let file_exists = fs::exists(path).map_err(|err| map_file_error(err, path, "check"))?;
    if file_exists {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("file {} already exists", path.display())));
    }

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| map_file_error(err, path, "open"))?;

    file.set_len(size as u64)
        .map_err(|err| map_file_error(err, path, "set length of"))?;

    file.lock().map_err(|err| map_file_error(err, path, "lock"))?;

    // SAFETY: The file is opened and locked; no other process can access it.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) }.map_err(|err| map_file_error(err, path, "mmap"))?;

    // The file is needed for the entire sniffer lifetime; it must not be dropped.
    std::mem::forget(file);

    return Ok(mmap);
}

fn run_controller(gpio: Gpio, cmd: ControllerCommand) -> io::Result<()> {
    info!("Start BlackGPiB v{VERSION} controller");

    match cmd {
        ControllerCommand::Status { address } => {
            let mut controller = DeviceController::new_with_reset(gpio, address);
            get_disk_status(&mut controller)?;
        }
        ControllerCommand::Format { address, validate } => {
            let mut controller = DeviceController::new_with_reset(gpio, address);
            get_disk_status(&mut controller)?;

            info!("Format disk...");

            let start = Instant::now();
            controller.format_disk(validate)?;

            info!("Disk successfully formatted in {:?}", start.elapsed());
        }
        ControllerCommand::Write {
            from_path: path,
            to_address: address,
        } => {
            let mut controller = DeviceController::new_with_reset(gpio, address);
            let disk_status = get_disk_status(&mut controller)?;
            let image_size = disk_status.size();

            let file = open_file_for_reading(&path, image_size)?;

            info!("Writing image to disk...");

            let start = Instant::now();
            controller.write_image_to_disk(file)?;

            info!("Image {} successfully written to disk in {:?}", path.display(), start.elapsed());
        }
        ControllerCommand::Read {
            from_address: address,
            to_path: path,
        } => {
            let mut controller = DeviceController::new_with_reset(gpio, address);
            let disk_status = get_disk_status(&mut controller)?;
            let image_size = disk_status.size();

            let file = open_file_for_writing(&path, image_size)?;

            info!("Reading disk to image file {}...", path.display());

            let start = Instant::now();
            controller.read_disk_to_writer(file)?;

            info!("Disk successfully read into file {} in {:?}", path.display(), start.elapsed());
        }
    }

    Ok(())
}

fn get_disk_status(controller: &mut DeviceController) -> io::Result<DiskIdentity> {
    info!("Read disk info");
    let disk_status = controller.read_status()?;

    info!("Device identified as:");
    info!("  Name: '{}'", disk_status.name());
    info!("  Sector Size: {}", disk_status.sector_size);
    info!("  Logical Sector Size: {}", disk_status.logical_sector_size);
    info!("  Disk Status: {}", disk_status.drive_status);
    info!("  Bitmap Block ID: {:#06x}", disk_status.bitmap_block_id);
    info!("  Superblock ID: {:#06x}", disk_status.superblock_id);
    info!("  Min Dir Pages: {}", disk_status.min_dir_pages);
    info!("  Flush: {}", disk_status.flush);
    info!("  Sector Count: {}", disk_status.sector_count);
    info!("  Bytes Per Sector: {}", disk_status.bytes_per_sector);
    info!("  Sectors Per Track: {}", disk_status.sectors_per_track);
    info!("  Tracks Per Cylinder: {}", disk_status.tracks_per_cylinder);
    info!("  Size (bytes): {}", disk_status.size());

    Ok(disk_status)
}

fn open_file_for_reading(path: &Path, file_size: usize) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(false)
        .read(true)
        .write(false)
        .open(path)
        .map_err(|err| map_file_error(err, path, "open"))?;

    file.lock().map_err(|err| map_file_error(err, path, "lock"))?;

    let file_metadata = file
        .metadata()
        .map_err(|err| map_file_error(err, path, "get metadata of"))?;

    if file_metadata.len() != file_size as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("file must be exactly {file_size} bytes long"),
        ));
    }

    Ok(file)
}

fn open_file_for_writing(path: &Path, file_size: usize) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(false)
        .write(true)
        .open(path)
        .map_err(|err| map_file_error(err, path, "open"))?;

    file.lock().map_err(|err| map_file_error(err, path, "lock"))?;

    file.set_len(file_size as u64)
        .map_err(|err| map_file_error(err, path, "set length of"))?;

    Ok(file)
}

fn map_file_error(err: io::Error, path: &Path, action: &str) -> io::Error {
    io::Error::new(err.kind(), format!("failed to {action} file {}: {}", path.display(), err))
}
