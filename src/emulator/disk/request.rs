#![allow(unused)]

use std::{fmt::Debug, mem::transmute};

pub const REQUEST_SIZE: usize = 10;

/// Request from GRiD Compass to disk.
#[derive(Debug)]
pub struct Request {
    /// Operation code. Determines what magic the emulator will do next.
    pub code: RequestCode,

    /// Unknown.
    pub connection: u16,

    /// Sector number. Only relevant for Initialize, Read, Write and Format operations.
    pub sector: u32,

    /// Request data size.
    /// For `GetStatus` and `Read`, this is the number of bytes the laptop expects in response.
    /// For `Write`, this is the size of data the laptop will send after this request.
    /// For other requests is unknown.
    pub data_size: u16,

    /// Unknown.
    pub mode: u8,
}

impl Request {
    pub fn into_bytes(self) -> [u8; REQUEST_SIZE] {
        let mut bytes = [0; REQUEST_SIZE];
        bytes[0] = self.code as u8;
        bytes[1..=2].copy_from_slice(&self.connection.to_le_bytes());
        bytes[3..=6].copy_from_slice(&self.sector.to_le_bytes());
        bytes[7..=8].copy_from_slice(&self.data_size.to_le_bytes());
        bytes[9] = self.mode;
        bytes
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = BadRequest;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != REQUEST_SIZE {
            return Err(BadRequest::InvalidLength { len: value.len() });
        }

        let Ok(command) = RequestCode::try_from(value[0]) else {
            return Err(BadRequest::UnsupportedRequest { code: value[0] });
        };

        Ok(Request {
            code: command,
            connection: u16::from_le_bytes([value[1], value[2]]),
            sector: u32::from_le_bytes([value[3], value[4], value[5], value[6]]),
            data_size: u16::from_le_bytes([value[7], value[8]]),
            mode: value[9],
        })
    }
}

/// All request codes from CCOS sources:
/// https://gridrepository.org/GRiD%20OS/Unknown%20Sources/OSINCS/driver.inc
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RequestCode {
    Initialize = 0,
    GetStatus = 1,

    Read = 4,
    Write = 5,

    SelfTest = 16,

    Format = 17,
    TrackFormat = 22,
}

impl TryFrom<u8> for RequestCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 | 2 | 4 | 5 | 16 | 17 | 22 => {
                // SAFETY: value is within valid range.
                Ok(unsafe { transmute::<u8, Self>(value) })
            }
            _ => {
                // NOTE: if you received this error on a real laptop, please create an issue.
                Err(value)
            }
        }
    }
}

/// Errors when parsing [`Request`] from a byte array.
#[derive(Debug)]
pub enum BadRequest {
    InvalidLength { len: usize },
    UnsupportedRequest { code: u8 },
}

impl std::fmt::Display for BadRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadRequest::InvalidLength { len } => {
                write!(f, "Invalid request length: got {}, expected {}", len, REQUEST_SIZE)
            }
            BadRequest::UnsupportedRequest { code } => {
                write!(f, "Unsupported request code {}", code)
            }
        }
    }
}

impl std::error::Error for BadRequest {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get_status_request() {
        let bytes: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::GetStatus);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector, 0);
        assert_eq!(request.data_size, 52);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_read_request() {
        let bytes: &[u8] = &[0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::Read);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector, 0);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_write_request() {
        let bytes: &[u8] = &[0x05, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x02, 0x01];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::Write);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector, 0xffffffff);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 1);
    }
}
