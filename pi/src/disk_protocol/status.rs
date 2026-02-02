/// Describes a block device connected to a GRiD computer.
#[derive(Clone, Copy, Default)]
pub struct Status {
    /// Actual sector size.
    ///
    /// Always 512 bytes.
    pub sector_size: u16,

    /// Logical sector size.
    ///
    /// Always 504 bytes.
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
    pub sector_count: u16,

    /// Status of the drive. 0 is not ready, 1 is ready, 3 is error.
    pub drive_status: u8,

    /// Bitmap block number.
    /// On the hard drive it's 0x2400, on the floppy drive it's 0x120.
    pub bitmap_block_id: u16,

    /// Superblock number.
    /// On the hard drive it's 0x2420, on the floppy drive it's 0x121.
    pub superblock_id: u16,

    /// Minimum number of sectors the system reserves for storing the file list
    /// in the Programs~Subject~.
    /// 
    /// On the hard drive it's 0x2400, on the floppy drive it's 0x120.
    pub min_dir_pages: u16,

    /// Unknown purpose.
    /// On the hard drive it's 0x0E, on the floppy drive it's 0.
    pub flush: u8,

    /// Device name. Not shown in the CCOS interface, can be anything.
    pub device_name: [u8; 32],

    /// Same as [`Self::sector_size`].
    pub bytes_per_sector: u16,

    /// Number of sectors per track.
    pub sectors_per_track: u16,

    /// Number of tracks per cylinder.
    pub tracks_per_cylinder: u16,
}

impl Status {
    /// Returns the device name as a human-readable string, removing any invalid UTF-8 bytes.
    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.device_name)
    }

    /// Returns total size in bytes of the disk.
    pub fn size(&self) -> usize {
        self.sector_size as usize * self.sector_count as usize
    }

    pub fn from_bytes(data: &[u8; 52]) -> Self {
        let mut device_name = [0u8; 32];
        device_name.copy_from_slice(&data[14..46]);

        Status {
            sector_size: u16::from_le_bytes([data[0], data[1]]),
            logical_sector_size: u16::from_le_bytes([data[2], data[3]]),
            sector_count: u16::from_le_bytes([data[4], data[5]]),
            drive_status: data[6],
            bitmap_block_id: u16::from_le_bytes([data[7], data[8]]),
            superblock_id: u16::from_le_bytes([data[9], data[10]]),
            min_dir_pages: u16::from_le_bytes([data[11], data[12]]),
            flush: data[13],
            device_name,
            bytes_per_sector: u16::from_le_bytes([data[46], data[47]]),
            sectors_per_track: u16::from_le_bytes([data[48], data[49]]),
            tracks_per_cylinder: u16::from_le_bytes([data[50], data[51]]),
        }
    }

    pub fn try_from_bytes(input: &[u8]) -> Result<Self, &[u8]> {
        if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes(data))
        } else {
            Err(input)
        }
    }

    pub fn into_bytes(self) -> [u8; 52] {
        let mut output = [0u8; 52];

        output[0..2].copy_from_slice(&self.sector_size.to_le_bytes());
        output[2..4].copy_from_slice(&self.logical_sector_size.to_le_bytes());
        output[4..6].copy_from_slice(&self.sector_count.to_le_bytes());
        output[6] = self.drive_status;
        output[7..9].copy_from_slice(&self.bitmap_block_id.to_le_bytes());
        output[9..11].copy_from_slice(&self.superblock_id.to_le_bytes());
        output[11..13].copy_from_slice(&self.min_dir_pages.to_le_bytes());
        output[13] = self.flush;
        output[14..46].copy_from_slice(&self.device_name);
        output[46..48].copy_from_slice(&self.bytes_per_sector.to_le_bytes());
        output[48..50].copy_from_slice(&self.sectors_per_track.to_le_bytes());
        output[50..52].copy_from_slice(&self.tracks_per_cylinder.to_le_bytes());

        output
    }
}

impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Status")
            .field("sector_size", &self.sector_size)
            .field("logical_sector_size", &self.logical_sector_size)
            .field("sector_count", &self.sector_count)
            .field("drive_status", &self.drive_status)
            .field("bitmap_block_id", &self.bitmap_block_id)
            .field("superblock_id", &self.superblock_id)
            .field("min_dir_pages", &self.min_dir_pages)
            .field("flush", &self.flush)
            .field("device_name", &self.name())
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("sectors_per_track", &self.sectors_per_track)
            .field("tracks_per_cylinder", &self.tracks_per_cylinder)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_hdd_status() {
        let bytes = [
            0x00, 0x02, 0xF8, 0x01, 0x8C, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x4D, 0x41, 0x4D, 0x45,
            0x20, 0x48, 0x41, 0x52, 0x44, 0x44, 0x49, 0x53, 0x4B, 0x20, 0x44, 0x52, 0x49, 0x56, 0x45, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x47, 0x52, 0x49, 0x44, 0x32, 0x31, 0x30, 0x31, 0x00, 0x02, 0x11, 0x00, 0x33, 0x01,
        ];

        let status = Status::from_bytes(&bytes);
        dbg!(status);

        assert_eq!(status.sector_size, 512);
        assert_eq!(status.logical_sector_size, 504);
        assert_eq!(status.sector_count, 20876);
        assert_eq!(status.drive_status, 1);
        assert_eq!(status.bitmap_block_id, 0);
        assert_eq!(status.superblock_id, 0);
        assert_eq!(status.min_dir_pages, 1);
        assert_eq!(status.flush, 0);
        assert_eq!(status.device_name, *b"MAME HARDDISK DRIVE     GRID2101");
        assert_eq!(status.bytes_per_sector, 512);
        assert_eq!(status.sectors_per_track, 17);
        assert_eq!(status.tracks_per_cylinder, 307);

        let serialized = status.into_bytes();
        assert_eq!(serialized, bytes);
    }

    #[test]
    fn test_round_trip_floppy_status() {
        let bytes = [
            0x00, 0x02, 0xF8, 0x01, 0xD0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38, 0x20, 0x54,
            0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4C, 0x4F, 0x50, 0x50, 0x59, 0x20, 0x20, 0x20,
            0x20, 0x33, 0x30, 0x30, 0x32, 0x33, 0x37, 0x2D, 0x30, 0x30, 0x00, 0x02, 0x09, 0x00, 0x02, 0x00,
        ];

        let status = Status::from_bytes(&bytes);
        dbg!(status);

        assert_eq!(status.sector_size, 512);
        assert_eq!(status.logical_sector_size, 504);
        assert_eq!(status.sector_count, 720);
        assert_eq!(status.drive_status, 1);
        assert_eq!(status.bitmap_block_id, 0x120);
        assert_eq!(status.superblock_id, 0x121);
        assert_eq!(status.min_dir_pages, 1);
        assert_eq!(status.flush, 0);
        assert_eq!(status.device_name, *b"48 TPI DS DD FLOPPY    300237-00");
        assert_eq!(status.bytes_per_sector, 512);
        assert_eq!(status.sectors_per_track, 9);
        assert_eq!(status.tracks_per_cylinder, 2);

        let serialized = status.into_bytes();
        assert_eq!(serialized[..52], bytes);
    }
}
