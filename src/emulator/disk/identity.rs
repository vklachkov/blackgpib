#![allow(unused)]

/// Describes a block device connected to a GRiD computer.
#[derive(Clone, Copy, Default)]
pub struct DiskIdentity {
    /// Actual sector size.
    ///
    /// For external disks always 512 bytes.
    ///
    /// In theory Compass supports 256 and 512 bytes per sector.
    /// But in reality, 512 bytes is hardcoded in the ROM and maybe in other places,
    /// and the GPIB state breaks after reading zero sector.
    pub sector_size: u16,

    /// Logical sector size.
    ///
    /// For 512 bytes per sector, it is 504 bytes; for 256, it is 252 bytes.
    ///
    /// Does not include 4 bytes for the ccos_block_header_t structure
    /// at the start of the sector.
    /// For 512 it also does not include 4 more bytes at the end of the sector,
    /// which in CCOS-disk-utils are called `block_end`.
    ///
    /// In source code it is called `logPageSize`.
    pub logical_sector_size: u16,

    /// Number of sectors.
    ///
    /// Calculated as disk size divided by [`Self::sector_size`].
    ///
    /// Must match real value, because CCOS checks
    /// disk boundaries when working with it. If files point to blocks
    /// outside the boundaries, the disk just will not be seen in the system.
    ///
    /// However, this is not important for MS-DOS 2.0 for Compass 110X.
    /// Because MS-DOS can easily try to read or write to a sector that
    /// is above the specified value.
    ///
    /// In source code it is called `numPages`.
    pub sector_count: u16,

    /// Unknown purpose. On real devices always true and
    /// the laptop does not react to changes for this field.
    pub drive_ready: bool,

    /// Bitmap block number. Usually 0x120 (one less than the superblock),
    /// but sometimes there are exceptions. Used only in CCOS.
    pub bitmap_block_id: u16,

    /// Superblock number. Usually 0x121, but sometimes there are exceptions.
    /// Used only in CCOS.
    pub superblock_id: u16,

    /// Unknown purpose. On real devices always 1.
    pub min_dir_pages: u16,

    /// Unknown purpose. On real devices always 0.
    pub flush: u8,

    /// Device name. Not shown in the CCOS interface, can be anything.
    pub device_name: [u8; 32],

    /// Same as [`Self::sector_size`].
    pub bytes_per_sector: u16,

    /// Unknown purpose.
    pub sectors_per_track: u16,

    /// Unknown purpose.
    pub tracks_per_cylinder: u16,

    /// Unknown purpose.
    pub interleave_factor: u8,

    /// Unknown purpose.
    pub second_side_count: u8,

    /// Unknown purpose.
    pub num_cylinders: u16,
}

impl DiskIdentity {
    /// Parses bytes into a struct. Supports parsing both the full 56-byte
    /// identifier and the shorter 52-byte identifier.
    pub fn from_bytes<const N: usize>(data: &[u8; N]) -> Self {
        const { assert!(N == 52 || N == 56, "invalid size") };

        let mut device_name = [0u8; 32];
        device_name.copy_from_slice(&data[14..46]);

        DiskIdentity {
            sector_size: u16::from_le_bytes([data[0], data[1]]),
            logical_sector_size: u16::from_le_bytes([data[2], data[3]]),
            sector_count: u16::from_le_bytes([data[4], data[5]]),
            drive_ready: data[6] == 1,
            bitmap_block_id: u16::from_le_bytes([data[7], data[8]]),
            superblock_id: u16::from_le_bytes([data[9], data[10]]),
            min_dir_pages: u16::from_le_bytes([data[11], data[12]]),
            flush: data[13],
            device_name,
            bytes_per_sector: u16::from_le_bytes([data[46], data[47]]),
            sectors_per_track: u16::from_le_bytes([data[48], data[49]]),
            tracks_per_cylinder: u16::from_le_bytes([data[50], data[51]]),
            interleave_factor: if N == 56 { data[52] } else { 0 },
            second_side_count: if N == 56 { data[53] } else { 0 },
            num_cylinders: if N == 56 {
                u16::from_le_bytes([data[54], data[55]])
            } else {
                0
            },
        }
    }

    /// Parses slice of bytes into a struct. If slice has invalid value,
    /// function returns Err(input).
    pub fn try_from_bytes(input: &[u8]) -> Result<Self, &[u8]> {
        if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes::<52>(data))
        } else if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes::<56>(data))
        } else {
            Err(input)
        }
    }

    /// Serializes the struct into a byte array.
    pub fn into_bytes(self) -> [u8; 56] {
        let mut output = [0u8; 56];

        output[0..2].copy_from_slice(&self.sector_size.to_le_bytes());
        output[2..4].copy_from_slice(&self.logical_sector_size.to_le_bytes());
        output[4..6].copy_from_slice(&self.sector_count.to_le_bytes());
        output[6] = self.drive_ready as u8;
        output[7..9].copy_from_slice(&self.bitmap_block_id.to_le_bytes());
        output[9..11].copy_from_slice(&self.superblock_id.to_le_bytes());
        output[11..13].copy_from_slice(&self.min_dir_pages.to_le_bytes());
        output[13] = self.flush;
        output[14..46].copy_from_slice(&self.device_name);
        output[46..48].copy_from_slice(&self.bytes_per_sector.to_le_bytes());
        output[48..50].copy_from_slice(&self.sectors_per_track.to_le_bytes());
        output[50..52].copy_from_slice(&self.tracks_per_cylinder.to_le_bytes());
        output[52] = self.interleave_factor;
        output[53] = self.second_side_count;
        output[54..56].copy_from_slice(&self.num_cylinders.to_le_bytes());

        output
    }
}

impl std::fmt::Debug for DiskIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskIdentity")
            .field("sector_size", &self.sector_size)
            .field("logical_sector_size", &self.logical_sector_size)
            .field("sector_count", &self.sector_count)
            .field("drive_ready", &self.drive_ready)
            .field("bitmap_block_id", &self.bitmap_block_id)
            .field("superblock_id", &self.superblock_id)
            .field("min_dir_pages", &self.min_dir_pages)
            .field("flush", &self.flush)
            .field("device_name", &String::from_utf8_lossy(&self.device_name))
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("sectors_per_track", &self.sectors_per_track)
            .field("tracks_per_cylinder", &self.tracks_per_cylinder)
            .field("interleave_factor", &self.interleave_factor)
            .field("second_side_count", &self.second_side_count)
            .field("num_cylinders", &self.num_cylinders)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_hdd_identity() {
        let bytes = [
            0x00, 0x02, 0xF8, 0x01, 0x8C, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x4D, 0x41, 0x4D, 0x45,
            0x20, 0x48, 0x41, 0x52, 0x44, 0x44, 0x49, 0x53, 0x4B, 0x20, 0x44, 0x52, 0x49, 0x56, 0x45, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x47, 0x52, 0x49, 0x44, 0x32, 0x31, 0x30, 0x31, 0x00, 0x02, 0x11, 0x00, 0x33, 0x01, 0x00, 0x00,
            0x04, 0x00,
        ];

        let status = DiskIdentity::from_bytes(&bytes);
        dbg!(status);

        assert_eq!(status.sector_size, 512);
        assert_eq!(status.logical_sector_size, 504);
        assert_eq!(status.sector_count, 20876);
        assert_eq!(status.drive_ready, true);
        assert_eq!(status.bitmap_block_id, 0b00000000);
        assert_eq!(status.superblock_id, 0);
        assert_eq!(status.min_dir_pages, 1);
        assert_eq!(status.flush, 0);
        assert_eq!(status.device_name, *b"MAME HARDDISK DRIVE     GRID2101");
        assert_eq!(status.bytes_per_sector, 512);
        assert_eq!(status.sectors_per_track, 17);
        assert_eq!(status.tracks_per_cylinder, 307);
        assert_eq!(status.interleave_factor, 0);
        assert_eq!(status.second_side_count, 0);
        assert_eq!(status.num_cylinders, 4);

        let serialized = status.into_bytes();
        assert_eq!(serialized, bytes);
    }

    #[test]
    fn test_round_trip_floppy_identity() {
        let bytes = [
            0x00, 0x02, 0xf8, 0x01, 0xD0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38, 0x20, 0x54,
            0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4c, 0x4f, 0x50, 0x50, 0x59, 0x20, 0x20, 0x20,
            0x20, 0x33, 0x30, 0x32, 0x33, 0x37, 0x2d, 0x30, 0x30, 0x00, 0x02, 0x09, 0x00, 0x09, 0x00, 0x02,
        ];

        let status = DiskIdentity::from_bytes(&bytes);
        dbg!(status);

        assert_eq!(status.sector_size, 512);
        assert_eq!(status.logical_sector_size, 504);
        assert_eq!(status.sector_count, 720);
        assert_eq!(status.drive_ready, true);
        assert_eq!(status.bitmap_block_id, 0b100100000);
        assert_eq!(status.superblock_id, 289);
        assert_eq!(status.min_dir_pages, 1);
        assert_eq!(status.flush, 0);
        assert_eq!(status.device_name, *b"48 TPI DS DD FLOPPY    30237-00\0");
        assert_eq!(status.bytes_per_sector, 2306);
        assert_eq!(status.sectors_per_track, 2304);
        assert_eq!(status.tracks_per_cylinder, 512);
        assert_eq!(status.interleave_factor, 0);
        assert_eq!(status.second_side_count, 0);
        assert_eq!(status.num_cylinders, 0);

        let serialized = status.into_bytes();
        assert_eq!(serialized[..52], bytes);
    }
}
