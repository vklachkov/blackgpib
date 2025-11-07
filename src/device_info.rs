use crate::{disk_identity::DiskIdentity, gpib::SupportedDeviceAddress};

pub const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::HardDisk;

pub const IDENTITY: [u8; 56] = DiskIdentity {
    // Disk parameters.
    sector_size: 512,
    logical_sector_size: 504,
    sector_count: 720,
    // Unknown.
    drive_ready: true,
    // Depends on image structure.
    bitmap_block_id: 0x120,
    superblock_id: 0x121,
    // Unknown.
    min_dir_pages: 1,
    flush: 0,
    //
    device_name: *b"48 TPI DS DD FLOPPY    30237-00\0",
    // Unknown. Extracted from real floppy. Weird values, but works.
    bytes_per_sector: 2306,
    sectors_per_track: 2304,
    tracks_per_cylinder: 512,
    // Unused by floppy.
    unknown: [0; 4],
}
.into_bytes();
