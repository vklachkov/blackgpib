use std::path::PathBuf;

use bpaf::{Parser, construct, long, params::ParseCommand, positional};

pub struct Args {
    pub verbose: bool,
    pub trace: bool,
    pub command: Command,
}

pub enum Command {
    Emulator(EmulatorArgs),
    Sniffer(SnifferArgs),
    Controller(ControllerArgs),
}

pub struct EmulatorArgs {
    pub hdd_1_image: Option<PathBuf>,
    pub floppy_drive_1_image: Option<PathBuf>,
    pub portable_floppy_image: Option<PathBuf>,
    pub hdd_2_image: Option<PathBuf>,
    pub floppy_drive_2_image: Option<PathBuf>,
}

pub struct SnifferArgs {
    pub output_path: PathBuf,
    pub size: usize,
}

pub struct ControllerArgs {
    pub address: u8,
}

impl Args {
    pub fn parse() -> Self {
        let verbose = long("verbose").help("Enable extra logs").switch();
        let trace = long("trace").help("Enable super mega verbose logs").switch();

        let emulator_cmd = Self::parse_emulator_command();
        let sniffer_cmd = Self::parse_sniffer_command();
        let controller_cmd = Self::parse_controller_command();
        let command = construct!([emulator_cmd, sniffer_cmd, controller_cmd]);

        construct!(Args {
            verbose,
            trace,
            command
        })
        .to_options()
        .descr("GPiB Peripheral Emulator for GRiD Compass")
        .run()
    }

    fn parse_emulator_command() -> ParseCommand<Command> {
        let hdd_1_image = long("hdd-1-image")
            .help("Image inserted to the first virtual HDD")
            .argument::<PathBuf>("PATH")
            .optional();

        let floppy_drive_1_image = long("floppy-drive-1-image")
            .help("Image inserted to the first virtual floppy drive")
            .argument::<PathBuf>("PATH")
            .optional();

        let portable_floppy_image = long("portable-floppy-image")
            .help("Image inserted to the portable floppy drive")
            .argument::<PathBuf>("PATH")
            .optional();

        let hdd_2_image = long("hdd-2-image")
            .help("Image inserted to the second virtual HDD")
            .argument::<PathBuf>("PATH")
            .optional();

        let floppy_drive_2_image = long("floppy-drive-2-image")
            .help("Image inserted to the second virtual floppy drive")
            .argument::<PathBuf>("PATH")
            .optional();

        let args = construct!(EmulatorArgs {
            hdd_1_image,
            floppy_drive_1_image,
            portable_floppy_image,
            hdd_2_image,
            floppy_drive_2_image,
        });

        construct!(Command::Emulator(args))
            .to_options()
            .descr("Emulate disk, printer and plotter")
            .command("emulator")
    }

    fn parse_sniffer_command() -> ParseCommand<Command> {
        let output_path = long("output")
            .help("Path to the file. If it does not exist, it will be created with `size` bytes")
            .argument::<PathBuf>("PATH");

        let size = long("size")
            .help("Maximum size of the dump file")
            .argument::<usize>("BYTES");

        let args = construct!(SnifferArgs { output_path, size });

        construct!(Command::Sniffer(args))
            .to_options()
            .descr("Capture communication between other devices")
            .command("sniffer")
    }

    fn parse_controller_command() -> ParseCommand<Command> {
        let address = positional("ADDRESS");

        let args = construct!(ControllerArgs { address });

        construct!(Command::Controller(args))
            .to_options()
            .descr("Communicate with peripheral devices like Compass")
            .command("controller")
    }
}
