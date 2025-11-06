use crate::{disk_identity::DiskIdentity, gpib::SupportedDeviceAddress};

pub const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::HardDisk;

pub const IDENTITY: [u8; 56] = DiskIdentity {
    sector_size: 512,
    log_sector_size: 504,
    sector_count: 720,
    drive_ready: true,
    bit_map: 0b100100000,
    dir_fid: 289,
    min_dir_pages: 1,
    flush: 0,
    dev_name: *b"48 TPI DS DD FLOPPY    30237-00\0",
    // Extracted from real floppy. Weird values, but works.
    bytes_per_sector: 2306,
    sectors_per_track: 2304,
    tracks_per_cylinder: 512,
    // Unused by floppy.
    unknown: [0; 4],
}.into_bytes();
