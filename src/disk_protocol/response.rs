const STATUS_RESPONSE_LEN: usize = 7;

#[derive(Clone, Copy, Debug)]
pub enum Response<'a> {
    Raw(&'a [u8]),
    Status(StatusResponse),
}

impl<'a> Response<'a> {
    pub fn ok(sector: Option<u32>) -> Self {
        Self::Status(StatusResponse::new(StatusResponseErrno::OK, sector.unwrap_or_default() as u16))
    }

    pub fn from_status(status: StatusResponseErrno, sector: u16) -> Self {
        Self::Status(StatusResponse::new(status, sector))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StatusResponse {
    /// Status code of the request.
    pub status: StatusResponseErrno,

    /// Exact purpose is unknown, maybe connection or drive init flag, on real drive always 0.
    pub unknown: u8,

    /// Sector number from the request, if needed.
    pub sector: u16,

    /// Unused, always 0.
    pub unused: u16,
}

impl StatusResponse {
    pub const fn new(status: StatusResponseErrno, sector: u16) -> Self {
        Self {
            status,
            unknown: 0,
            sector,
            unused: 0,
        }
    }

    pub fn from_bytes(input: &[u8; STATUS_RESPONSE_LEN]) -> Self {
        Self {
            status: StatusResponseErrno(u16::from_le_bytes([input[0], input[1]])),
            unknown: input[1],
            sector: u16::from_le_bytes([input[3], input[4]]),
            unused: u16::from_le_bytes([input[5], input[6]]),
        }
    }

    pub fn try_from_bytes(input: &[u8]) -> Result<Self, &[u8]> {
        if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes(data))
        } else {
            Err(input)
        }
    }

    pub fn into_bytes(self) -> [u8; STATUS_RESPONSE_LEN] {
        let mut bytes = [0; STATUS_RESPONSE_LEN];
        bytes[0..=1].copy_from_slice(&self.status.0.to_le_bytes());
        bytes[2] = self.unknown;
        bytes[3..=4].copy_from_slice(&self.sector.to_le_bytes());
        bytes[5..=6].copy_from_slice(&self.unused.to_le_bytes());
        bytes
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StatusResponseErrno(pub u16);

impl StatusResponseErrno {
    pub const OK: Self = Self(0x00);

    pub const UNSUPPORTED: Self = Self(0x23);

    pub const NOT_READY: Self = Self(0x6b);

    pub const OUT_OF_BOUNDS: Self = Self(0x66);

    pub const BAD_SECTOR: Self = Self(0x67);

    pub const NOT_FORMATTED: Self = Self(0x68);
}

impl std::fmt::Debug for StatusResponseErrno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::OK => write!(f, "OK"),
            Self::UNSUPPORTED => write!(f, "Unsupported"),
            Self::NOT_READY => write!(f, "NotReady"),
            Self::OUT_OF_BOUNDS => write!(f, "OutOfBounds"),
            Self::BAD_SECTOR => write!(f, "BadSector"),
            Self::NOT_FORMATTED => write!(f, "NotFormatted"),
            Self(code) => write!(f, "Errno({:#04X})", code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_response_into_bytes() {
        let status = StatusResponseErrno::NOT_FORMATTED;
        let sector = 0xABCD;
        let response = StatusResponse::new(status, sector);

        let bytes = response.into_bytes();

        assert_eq!(bytes.len(), STATUS_RESPONSE_LEN);
        assert_eq!(bytes, [0x68, 0x00, 0x00, 0xCD, 0xAB, 0x00, 0x00]);
    }

    #[test]
    fn test_status_response_from_bytes() {
        let bytes = [0x68, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD];
        let response = StatusResponse::from_bytes(&bytes);

        assert_eq!(response.status, StatusResponseErrno::NOT_FORMATTED);
        assert_eq!(response.unknown, 0);
        assert_eq!(response.sector, 0xBBAA);
        assert_eq!(response.unused, 0xDDCC);
    }

    #[test]
    fn test_status_response_try_from_bytes() {
        let bytes = [0; STATUS_RESPONSE_LEN];
        let result = StatusResponse::try_from_bytes(&bytes);
        assert!(result.is_ok());

        let bytes = [0; 2];
        let result = StatusResponse::try_from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_status_response_round_trip() {
        let original = StatusResponse::new(StatusResponseErrno::BAD_SECTOR, 0x1337);
        let bytes = original.into_bytes();
        let reparsed = StatusResponse::from_bytes(&bytes);

        assert_eq!(original.status, reparsed.status);
        assert_eq!(original.sector, reparsed.sector);
        assert_eq!(original.unknown, reparsed.unknown);
        assert_eq!(original.unused, reparsed.unused);
    }
}
