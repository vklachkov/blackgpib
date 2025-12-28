#![allow(unused)]

use std::fmt::Debug;

pub enum Response<'a> {
    Raw(&'a [u8]),
    Status([u8; 7]),
}

impl<'a> Response<'a> {
    pub fn ok(sector: Option<u32>) -> Self {
        let response = StatusResponse::new(DiskStatus::Ok, sector.unwrap_or_default() as u16);
        Self::Status(response.as_bytes())
    }

    pub fn from_status(status: DiskStatus, sector: u16) -> Self {
        Self::Status(StatusResponse::new(status, sector).as_bytes())
    }

    pub fn as_slice(&'a self) -> &'a [u8] {
        match self {
            Response::Raw(bytes) => bytes,
            Response::Status(bytes) => bytes,
        }
    }
}

/// Status response from disk.
#[derive(Clone, Copy, Debug)]
pub struct StatusResponse {
    /// Status code of the request.
    status: DiskStatus,

    /// Exact purpose is unknown, maybe connection or drive init flag, on real drive always 0.
    unknown1: u8,

    /// Sector size from the request, if needed.
    sector: u16,

    /// Unused, always 0.
    unused: u16,
}

impl StatusResponse {
    pub const fn new(status: DiskStatus, sector: u16) -> Self {
        Self {
            status,
            unknown1: 0,
            sector,
            unused: 0,
        }
    }

    pub fn from_bytes(input: &[u8; 7]) -> Self {
        Self {
            status: (((input[1] as u16) << 8) | (input[0] as u16)).into(),
            unknown1: input[1],
            sector: ((input[4] as u16) << 8) | (input[3] as u16),
            unused: ((input[6] as u16) << 8) | (input[5] as u16),
        }
    }

    pub fn try_from_bytes(input: &[u8]) -> Result<Self, &[u8]> {
        if let Ok(data) = input.try_into() {
            Ok(Self::from_bytes(data))
        } else {
            Err(input)
        }
    }

    #[inline]
    pub fn as_bytes(self) -> [u8; 7] {
        [
            ((Into::<u16>::into(self.status) & 0x00FF) >> 0) as u8,
            ((Into::<u16>::into(self.status) & 0xFF00) >> 8) as u8,
            self.unknown1,
            ((self.sector & 0x00FF) >> 0) as u8,
            ((self.sector & 0xFF00) >> 8) as u8,
            ((self.unused & 0x00FF) >> 0) as u8,
            ((self.unused & 0xFF00) >> 8) as u8,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u16)]
pub enum DiskStatus {
    Ok,
    UnsupportedCommand,
    NotReady,
    OutOfBounds,
    BadSector,
    NotFormatted,
    Unsupported(u16),
}

impl Into<u16> for DiskStatus {
    fn into(self) -> u16 {
        match self {
            Self::Ok => 0x00,
            Self::UnsupportedCommand => 0x23,
            Self::NotReady => 0x6b,
            Self::OutOfBounds => 0x66,
            Self::BadSector => 0x67,
            Self::NotFormatted => 0x68,
            Self::Unsupported(value) => value,
        }
    }
}

impl From<u16> for DiskStatus {
    fn from(value: u16) -> Self {
        match value {
            0x00 => Self::Ok,
            0x23 => Self::UnsupportedCommand,
            0x6b => Self::NotReady,
            0x66 => Self::OutOfBounds,
            0x67 => Self::BadSector,
            0x68 => Self::NotFormatted,
            _ => Self::Unsupported(value),
        }
    }
}
