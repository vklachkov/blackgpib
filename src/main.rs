#![allow(clippy::needless_return, clippy::upper_case_acronyms)]

mod args;
mod controller;
mod emulator;
mod gpib;
mod gpio;
mod logger;
mod sniffer;
mod system;
mod time_utils;

use std::{fs, io, path::Path, process::ExitCode};

use crate::{
    args::{Args, ControllerArgs, EmulatorArgs, SnifferArgs},
    controller::DeviceController,
    emulator::DeviceEmulator,
    gpio::Gpio,
    logger::LogLevel,
    sniffer::BusSniffer,
    system::{DeviceInfo, GpioInterface},
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
        args::Command::Emulator(args) => run_emulator(args, gpio),
        args::Command::Sniffer(args) => run_sniffer(args, gpio),
        args::Command::Controller(args) => run_controller(args),
    }
}

fn check_device_compatibility() -> io::Result<()> {
    debug!("Check device compatibility");

    match DeviceInfo::new() {
        Ok(info) => match info.gpio_interface() {
            GpioInterface::Bcm => {
                info!("Detected supported {} ({})", info.model(), info.soc());
                return Ok(());
            }
            GpioInterface::Rp1 => {
                info!("Sorry, your {} ({}) does not supported :(", info.model(), info.soc());
                return Err(io::Error::new(io::ErrorKind::Unsupported, "RP1 gpio does not supported"));
            }
        },
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "Unknown or unsupported Raspberry Pi model"));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

fn open_gpio() -> io::Result<Gpio> {
    debug!("Open GPIO");

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

fn run_emulator(args: EmulatorArgs, gpio: Gpio) -> io::Result<()> {
    let mut emulator = DeviceEmulator::new();

    debug!("Configure emulator before start");
    configure_emulator(args, &mut emulator)?;

    info!("Start BlackGPiB v{VERSION} emulator");
    emulator.start(gpio);

    Ok(())
}

fn configure_emulator(args: EmulatorArgs, emulator: &mut DeviceEmulator) -> io::Result<()> {
    emulator.create_proxy(21, 49274); // default printer
    emulator.create_proxy(25, 49275); // printer hp
    emulator.create_proxy(20, 49276); // plotter

    if let Some(ref path) = args.hdd_1_image {
        emulator.create_disk(04, mmap_disk_image(path)?);
    }
    if let Some(ref path) = args.floppy_drive_1_image {
        emulator.create_disk(05, mmap_disk_image(path)?);
    }
    if let Some(ref path) = args.portable_floppy_image {
        emulator.create_disk(06, mmap_disk_image(path)?);
    }
    if let Some(ref path) = args.hdd_2_image {
        emulator.create_disk(12, mmap_disk_image(path)?);
    }
    if let Some(ref path) = args.floppy_drive_2_image {
        emulator.create_disk(13, mmap_disk_image(path)?);
    }

    Ok(())
}

fn mmap_disk_image(path: &Path) -> io::Result<memmap2::MmapMut> {
    let map_err = |err: io::Error, action: &str| {
        io::Error::new(err.kind(), format!("failed to {action} file {}: {}", path.display(), err))
    };

    debug!("Open disk image {}", path.display());

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .truncate(false)
        .open(path)
        .map_err(|err| map_err(err, "open"))?;

    file.lock().map_err(|err| map_err(err, "lock"))?;

    // SAFETY: The file is opened and locked; no other process can access it.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) }.map_err(|err| map_err(err, "mmap"))?;

    // The file is needed for the entire emulator lifetime; it must not be dropped.
    std::mem::forget(file);

    return Ok(mmap);
}

fn run_sniffer(args: SnifferArgs, gpio: Gpio) -> io::Result<()> {
    let file = create_dump_file(&args.output_path, args.size)?;
    let sniffer = BusSniffer::new(file);

    info!("Start BlackGPiB v{VERSION} sniffer");

    sniffer.start(gpio);

    info!("Bus sniffer finished, dump saved to {}", args.output_path.display());

    Ok(())
}

fn create_dump_file(path: &Path, size: usize) -> io::Result<memmap2::MmapMut> {
    let map_err = |err: io::Error, action: &str| {
        io::Error::new(err.kind(), format!("failed to {action} file {}: {}", path.display(), err))
    };

    let file_exists = fs::exists(path).map_err(|err| map_err(err, "check"))?;
    if file_exists {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("file {} already exists", path.display())));
    }

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| map_err(err, "open"))?;

    file.set_len(size as u64).map_err(|err| map_err(err, "set length of"))?;

    file.lock().map_err(|err| map_err(err, "lock"))?;

    // SAFETY: The file is opened and locked; no other process can access it.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&file) }.map_err(|err| map_err(err, "mmap"))?;

    // The file is needed for the entire sniffer lifetime; it must not be dropped.
    std::mem::forget(file);

    return Ok(mmap);
}

fn run_controller(args: ControllerArgs) -> io::Result<()> {
    let controller = DeviceController::new(args.address);

    info!("Start BlackGPiB v{VERSION} controller");
    controller.start();

    Ok(())
}
