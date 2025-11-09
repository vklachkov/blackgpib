/// Describes a block device connected to a GRiD computer.
#[derive(Clone, Copy, Default)]
pub struct DiskIdentity {
    // also known as pageSize.
    pub sector_size: u16,
    pub logical_sector_size: u16,
    // also known as numPages.
    pub sector_count: u16,
    pub drive_ready: bool,
    pub bitmap_block_id: u16,
    // address of superblock
    pub superblock_id: u16,
    pub min_dir_pages: u16,
    pub flush: u8,
    pub device_name: [u8; 32],
    pub bytes_per_sector: u16,
    pub sectors_per_track: u16,
    pub tracks_per_cylinder: u16,
    pub interleave_factor: u8,
    pub second_side_count: u8,
    pub num_cylinders: u16,
}

impl DiskIdentity {
    #[cfg(test)]
    pub const fn from_bytes<const N: usize>(data: &[u8; N]) -> Self {
        const { assert!(N == 52 || N == 56, "invalid size") };

        let mut status = unsafe { core::mem::zeroed::<DiskIdentity>() };

        status.sector_size = u16::from_le_bytes([data[0], data[1]]);
        status.logical_sector_size = u16::from_le_bytes([data[2], data[3]]);
        status.sector_count = u16::from_le_bytes([data[4], data[5]]);
        status.drive_ready = data[6] == 1;
        status.bitmap_block_id = u16::from_le_bytes([data[7], data[8]]);
        status.superblock_id = u16::from_le_bytes([data[9], data[10]]);
        status.min_dir_pages = u16::from_le_bytes([data[11], data[12]]);
        status.flush = data[13];

        let mut i = 0;
        while i != 32 {
            status.device_name[i] = data[14 + i];
            i += 1;
        }

        status.bytes_per_sector = u16::from_le_bytes([data[46], data[47]]);
        status.sectors_per_track = u16::from_le_bytes([data[48], data[49]]);
        status.tracks_per_cylinder = u16::from_le_bytes([data[50], data[51]]);

        if N == 56 {
            status.interleave_factor = data[52];
            status.second_side_count = data[53];
            status.num_cylinders = u16::from_le_bytes([data[54], data[55]]);
        } else {
            status.interleave_factor = 0;
            status.second_side_count = 0;
            status.num_cylinders = 0;
        }

        status
    }

    pub const fn into_bytes(self) -> [u8; 56] {
        let mut output = [0; 56];

        let bytes = self.sector_size.to_le_bytes();
        output[0] = bytes[0];
        output[1] = bytes[1];

        let bytes = self.logical_sector_size.to_le_bytes();
        output[2] = bytes[0];
        output[3] = bytes[1];

        let bytes = self.sector_count.to_le_bytes();
        output[4] = bytes[0];
        output[5] = bytes[1];

        output[6] = self.drive_ready as u8;

        let bytes = self.bitmap_block_id.to_le_bytes();
        output[7] = bytes[0];
        output[8] = bytes[1];

        let bytes = self.superblock_id.to_le_bytes();
        output[9] = bytes[0];
        output[10] = bytes[1];

        let bytes = self.min_dir_pages.to_le_bytes();
        output[11] = bytes[0];
        output[12] = bytes[1];

        output[13] = self.flush;

        let mut i = 0;
        while i < 32 {
            output[14 + i] = self.device_name[i];
            i += 1;
        }

        let bytes = self.bytes_per_sector.to_le_bytes();
        output[46] = bytes[0];
        output[47] = bytes[1];

        let bytes = self.sectors_per_track.to_le_bytes();
        output[48] = bytes[0];
        output[49] = bytes[1];

        let bytes = self.tracks_per_cylinder.to_le_bytes();
        output[50] = bytes[0];
        output[51] = bytes[1];

        output[52] = self.interleave_factor;
        output[53] = self.second_side_count;

        let bytes = self.num_cylinders.to_le_bytes();
        output[54] = bytes[0];
        output[55] = bytes[1];

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
