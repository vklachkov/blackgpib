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
    Controller(ControllerCommand),
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

pub enum ControllerCommand {
    Status { address: u8 },
    Format { validate: bool, address: u8 },
    Write { from_path: PathBuf, to_address: u8 },
    Read { from_address: u8, to_path: PathBuf },
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
        .descr("GPIB Peripheral Emulator for GRiD Compass")
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
        // -- Disk status ----------------------------------------------------------------------------------------------

        let address = positional("ADDRESS")
            .help("GPIB device bus address")
            .guard(|v| (0..=30).contains(v), "address must be in range 0..30");

        let status_cmd = construct!(ControllerCommand::Status { address })
            .to_options()
            .descr("Show device status")
            .command("status");

        // -- Format disk ----------------------------------------------------------------------------------------------

        let validate = long("validate").help("Verify sectors after format").switch();

        let address = positional("ADDRESS")
            .help("GPIB device bus address")
            .guard(|v| (0..=30).contains(v), "address must be in range 0..30");

        let format_cmd = construct!(ControllerCommand::Format { validate, address })
            .to_options()
            .descr("Low level disk format")
            .command("format");

        // -- Read disk ------------------------------------------------------------------------------------------------

        let from_address = long("from")
            .help("GPIB device bus address")
            .argument("ADDRESS")
            .guard(|v| (0..=30).contains(v), "address must be in range 0..30");

        let to_path = long("to").help("Output image path").argument("IMAGE_PATH");

        let read_cmd = construct!(ControllerCommand::Read { from_address, to_path })
            .to_options()
            .descr("Read disk image from disk")
            .command("read");

        // -- Write disk -----------------------------------------------------------------------------------------------

        let from_path = long("from").help("Input image path").argument("IMAGE_PATH");

        let to_address = long("to")
            .help("GPIB device bus address")
            .argument("ADDRESS")
            .guard(|v| (0..=30).contains(v), "address must be in range 0..30");

        let write_cmd = construct!(ControllerCommand::Write { from_path, to_address })
            .to_options()
            .descr("Write disk image")
            .command("write");

        // -- Collect subcommands to parser ----------------------------------------------------------------------------

        let subcommands = construct!([status_cmd, format_cmd, read_cmd, write_cmd]);

        construct!(Command::Controller(subcommands))
            .to_options()
            .descr("Communicate with peripheral devices like Compass")
            .command("controller")
    }
}
