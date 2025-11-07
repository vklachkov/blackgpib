use crate::{disk_identity::DiskIdentity, gpib::SupportedDeviceAddress};

pub const ADDRESS: SupportedDeviceAddress = SupportedDeviceAddress::ExternalFloppy;

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
    device_name: *b"48 TPI DS DD FLOPPY    300237-00",
    bytes_per_sector: 512,
    sectors_per_track: 9,
    tracks_per_cylinder: 2,
    // Unused by floppy.
    interleave_factor: 0,
    second_side_count: 0,
    num_cylinders: 0,
}
.into_bytes();
