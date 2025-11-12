use std::path::PathBuf;

use bpaf::{Parser, construct, long};

#[derive(Debug)]
pub struct Args {
    pub trace: bool,
    pub verbose: bool,

    pub hdd_1_image: Option<PathBuf>,
    pub floppy_drive_1_image: Option<PathBuf>,
    pub portable_floppy_image: Option<PathBuf>,
    pub hdd_2_image: Option<PathBuf>,
    pub floppy_drive_2_image: Option<PathBuf>,
}

impl Args {
    pub fn parse() -> Self {
        let trace = long("trace").help("Enable super mega verbose logs").switch();
        let verbose = long("verbose").help("Enable extra logs").switch();

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

        let parser = construct!(Args {
            trace,
            verbose,
            hdd_1_image,
            floppy_drive_1_image,
            portable_floppy_image,
            hdd_2_image,
            floppy_drive_2_image,
        });

        let args = parser.to_options().descr("GPiB Peripheral Emulator for GRiD Compass");

        args.run()
    }
}
