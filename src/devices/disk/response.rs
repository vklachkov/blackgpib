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

    /// Unknown, always 0.
    unknown1: u16,

    /// Sector size from the request.
    sector: u16,

    /// Unknown, always 0.
    unknown2: u16,
}

impl StatusResponse {
    pub const fn new(status: DiskStatus, sector: u16) -> Self {
        Self {
            status,
            unknown1: 0,
            sector,
            unknown2: 0,
        }
    }

    #[inline]
    pub const fn as_bytes(self) -> [u8; 7] {
        [
            self.status as u8,
            ((self.unknown1 & 0x00FF) >> 0) as u8,
            ((self.unknown1 & 0xFF00) >> 8) as u8,
            ((self.sector & 0x00FF) >> 0) as u8,
            ((self.sector & 0xFF00) >> 8) as u8,
            ((self.unknown2 & 0x00FF) >> 0) as u8,
            ((self.unknown2 & 0xFF00) >> 8) as u8,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum DiskStatus {
    Ok = 0x00,
    NotReady = 0x6b,
    BadSector = 0x67,
    NotFormated = 0x68,
}
