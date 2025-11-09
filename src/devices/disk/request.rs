#![allow(unused)]

use std::{fmt::Debug, mem::transmute};

pub const REQUEST_SIZE: usize = 10;

/// Request from GRiD Compass to disk.
#[derive(Debug)]
pub struct Request {
    /// Operation code. Determines what magic the emulator will do next.
    pub code: RequestCode,

    /// Device number on address. Not supported by emulator, unknown how it works.
    /// Valid values are 0 or 1, other values cause failures according to usernameak.
    pub connection: u16,

    /// Sector number. Only relevant for Read and Write operations.
    pub sector_number: u32,

    /// Request data size.
    /// For `GetStatus` and `Read`, this is the number of bytes the laptop expects in response.
    /// For `Write`, this is the size of data the laptop will send after this request.
    pub data_size: u16,

    /// Unknown, unused.
    pub mode: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum BadRequest {
    #[error("Invalid request length: got {len}, expected {REQUEST_SIZE}")]
    InvalidLength { len: usize },

    #[error("Unsupported request code {code}")]
    UnsupportedRequest { code: u8 },
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
            sector_number: u32::from_le_bytes([value[3], value[4], value[5], value[6]]),
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
    Open = 2,
    Close = 3,
    Read = 4,
    Write = 5,
    Seek = 6,
    Truncate = 7,
    Attach = 8,
    Detach = 9,
    Rename = 10,
    Delete = 11,
    ReadDesc = 12,
    WriteDesc = 13,
    Flush = 14,
    WaitSRQ = 15,
    SelfTest = 16,
    Format = 17,
    SetStatus = 20,
    Deactivate = 21,

    TrackFormat = 22,
    ControllerTest = 23,
    RamTest = 24,
    DriveTest = 25,
    Prog = 26,
    WriteProtect = 27,
    BufferCommand = 28,
    ReadDirPage = 29,
    Signon = 30,
    SignOff = 31,
    Send = 32,
    RemoteCopy = 33,
    VerifyMedia = 40,
    ReadVolName = 41,
    AddMassVolName = 42,

    Connect = 100,
    DisConnect = 101,
    WaitConnect = 102,
}

impl TryFrom<u8> for RequestCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=17 | 20..=33 | 40..=42 | 100..=102 => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get_status_request() {
        let bytes: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::GetStatus);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector_number, 0);
        assert_eq!(request.data_size, 52);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_read_request() {
        let bytes: &[u8] = &[0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::Read);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector_number, 0);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 0);
    }

    #[test]
    fn parse_write_request() {
        let bytes: &[u8] = &[0x05, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x02, 0x01];
        let request = Request::try_from(bytes).expect("Failed to decode request");

        assert_eq!(request.code, RequestCode::Write);
        assert_eq!(request.connection, 0);
        assert_eq!(request.sector_number, 0xffffffff);
        assert_eq!(request.data_size, 512);
        assert_eq!(request.mode, 1);
    }
}
