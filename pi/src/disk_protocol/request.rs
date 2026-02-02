const REQUEST_SIZE: usize = 10;

#[derive(Debug)]
pub struct Request {
    /// Operation code. Determines what magic the emulator will do next.
    pub code: RequestCode,

    /// Unknown.
    pub unknown1: u8,

    /// Unknown.
    pub unknown2: u8,

    /// Sector number.
    pub sector: u32,

    /// Request data size.
    /// For Format, it must be 1.
    /// For GetStatus, it can be 52 or 54.
    /// For Read and Write, it should always be 512.
    pub data_size: u16,

    /// Determines what action the command will do.
    /// For example, Write with mode=1 is a verification of the received data.
    /// Or, for SelfTest, mode=7 turns the drive on, mode=8 turns the drive power off.
    pub mode: u8,
}

impl Request {
    pub fn new(code: RequestCode, sector: Option<u32>, data_size: u16) -> Self {
        Self {
            code,
            unknown1: 0,
            unknown2: 0,
            sector: sector.unwrap_or_default(),
            data_size,
            mode: 0,
        }
    }

    pub fn from_bytes(input: &[u8; REQUEST_SIZE]) -> Self {
        Self {
            code: RequestCode(input[0]),
            unknown1: input[1],
            unknown2: input[2],
            sector: u32::from_le_bytes([input[3], input[4], input[5], input[6]]),
            data_size: u16::from_le_bytes([input[7], input[8]]),
            mode: input[9],
        }
    }

    pub fn try_from_bytes(input: &[u8]) -> Result<Self, &[u8]> {
        if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes(data))
        } else {
            Err(input)
        }
    }

    pub fn into_bytes(self) -> [u8; REQUEST_SIZE] {
        let mut bytes = [0; REQUEST_SIZE];
        bytes[0] = self.code.0;
        bytes[1] = self.unknown1;
        bytes[2] = self.unknown2;
        bytes[3..=6].copy_from_slice(&self.sector.to_le_bytes());
        bytes[7..=8].copy_from_slice(&self.data_size.to_le_bytes());
        bytes[9] = self.mode;
        bytes
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RequestCode(u8);

// All request codes from CCOS sources:
// https://gridrepository.org/GRiD%20OS/Unknown%20Sources/OSINCS/driver.inc
impl RequestCode {
    pub const INITIALIZE: Self = Self(0);
    pub const GET_STATUS: Self = Self(1);

    pub const READ: Self = Self(4);
    pub const WRITE: Self = Self(5);

    pub const SELF_TEST: Self = Self(16);

    pub const FORMAT: Self = Self(17);
    pub const TRACK_FORMAT: Self = Self(22);
}

impl std::fmt::Debug for RequestCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::INITIALIZE => write!(f, "Initialize"),
            Self::GET_STATUS => write!(f, "GetStatus"),
            Self::READ => write!(f, "Read"),
            Self::WRITE => write!(f, "Write"),
            Self::SELF_TEST => write!(f, "SelfTest"),
            Self::FORMAT => write!(f, "Format"),
            Self::TRACK_FORMAT => write!(f, "TrackFormat"),
            Self(code) => write!(f, "Request({code:#04X})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get_status_request() {
        let bytes: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00];
        let request = Request::try_from_bytes(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::GET_STATUS);
        assert_eq!(request.unknown1, 0);
        assert_eq!(request.unknown2, 0);
        assert_eq!(request.sector, 0);
        assert_eq!(request.data_size, 52);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_read_request() {
        let bytes: &[u8] = &[0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
        let request = Request::try_from_bytes(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::READ);
        assert_eq!(request.unknown1, 0);
        assert_eq!(request.unknown2, 0);
        assert_eq!(request.sector, 0);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_write_request() {
        let bytes: &[u8] = &[0x05, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x02, 0x01];
        let request = Request::try_from_bytes(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::WRITE);
        assert_eq!(request.unknown1, 0);
        assert_eq!(request.unknown2, 0);
        assert_eq!(request.sector, 0xffffffff);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 1);
    }
}
