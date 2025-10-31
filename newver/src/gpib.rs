#[derive(Clone, Copy)]
pub enum GPIB {
    /// Transfer Enable/Talk Enable (SN7516x).
    TE,
    /// Direction Control (SN75161).
    DC,
    /// System Control (SN75162).
    SC,

    /// Data Input/Output, bit 1.
    DIO1,
    /// Data Input/Output, bit 2.
    DIO2,
    /// Data Input/Output, bit 3.
    DIO3,
    /// Data Input/Output, bit 4.
    DIO4,
    /// Data Input/Output, bit 5.
    DIO5,
    /// Data Input/Output, bit 6.
    DIO6,
    /// Data Input/Output, bit 7.
    DIO7,
    /// Data Input/Output, bit 8.
    DIO8,

    /// Data Available.
    DAV,
    /// Not Ready For Data.
    NRFD,
    /// Not Data Accepted.
    NDAC,

    /// Attention.
    ATN,
    /// Interface Clear.
    IFC,
    /// Service Request.
    SRQ,
    /// Remote Enable.
    REN,
    /// End Or Identify.
    EOI
}

impl GPIB {
    pub const fn all() -> [GPIB; 19] {
        [
            GPIB::TE,
            GPIB::DC,
            GPIB::SC,
            GPIB::DIO1,
            GPIB::DIO2,
            GPIB::DIO3,
            GPIB::DIO4,
            GPIB::DIO5,
            GPIB::DIO6,
            GPIB::DIO7,
            GPIB::DIO8,
            GPIB::DAV,
            GPIB::NRFD,
            GPIB::NDAC,
            GPIB::ATN,
            GPIB::IFC,
            GPIB::SRQ,
            GPIB::REN,
            GPIB::EOI,
        ]
    }

    pub const fn data() -> [GPIB; 8] {
        [
            GPIB::DIO8,
            GPIB::DIO7,
            GPIB::DIO6,
            GPIB::DIO5,
            GPIB::DIO4,
            GPIB::DIO3,
            GPIB::DIO2,
            GPIB::DIO1,
        ]
    }

    pub const fn pin_number(self) -> u8 {
        match self {
            GPIB::TE => 7,
            GPIB::DC => 11,
            GPIB::SC => 17,
            GPIB::DIO1 => 5,
            GPIB::DIO2 => 6,
            GPIB::DIO3 => 12,
            GPIB::DIO4 => 13,
            GPIB::DIO5 => 19,
            GPIB::DIO6 => 16,
            GPIB::DIO7 => 26,
            GPIB::DIO8 => 20,
            GPIB::DAV => 10,
            GPIB::NRFD => 24,
            GPIB::NDAC => 22,
            GPIB::ATN => 9,
            GPIB::IFC => 23,
            GPIB::SRQ => 8,
            GPIB::REN => 27,
            GPIB::EOI => 25,
        }
    }
}