#[derive(Clone, Copy)]
pub enum GPIBPin {
    /// Transfer Enable/Talk Enable (SN7516x).
    TE,
    /// Direction Control (SN75161 and 162).
    DC,
    /// System Control (SN75162 only).
    SC,
    /// System Control (SN75161 and 162).
    PE,

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
    EOI,
}

impl GPIBPin {
    pub const fn all() -> [GPIBPin; 20] {
        [
            GPIBPin::TE,
            GPIBPin::DC,
            GPIBPin::SC,
            GPIBPin::PE,
            GPIBPin::DIO1,
            GPIBPin::DIO2,
            GPIBPin::DIO3,
            GPIBPin::DIO4,
            GPIBPin::DIO5,
            GPIBPin::DIO6,
            GPIBPin::DIO7,
            GPIBPin::DIO8,
            GPIBPin::DAV,
            GPIBPin::NRFD,
            GPIBPin::NDAC,
            GPIBPin::ATN,
            GPIBPin::IFC,
            GPIBPin::SRQ,
            GPIBPin::REN,
            GPIBPin::EOI,
        ]
    }

    pub const fn data() -> [GPIBPin; 8] {
        [
            GPIBPin::DIO8,
            GPIBPin::DIO7,
            GPIBPin::DIO6,
            GPIBPin::DIO5,
            GPIBPin::DIO4,
            GPIBPin::DIO3,
            GPIBPin::DIO2,
            GPIBPin::DIO1,
        ]
    }

    pub const fn pin_number(self) -> u8 {
        match self {
            GPIBPin::TE => 7,
            GPIBPin::DC => 11,
            GPIBPin::SC => 17,
            GPIBPin::PE => 21,
            GPIBPin::DIO1 => 5,
            GPIBPin::DIO2 => 6,
            GPIBPin::DIO3 => 12,
            GPIBPin::DIO4 => 13,
            GPIBPin::DIO5 => 19,
            GPIBPin::DIO6 => 16,
            GPIBPin::DIO7 => 26,
            GPIBPin::DIO8 => 20,
            GPIBPin::DAV => 10,
            GPIBPin::NRFD => 24,
            GPIBPin::NDAC => 22,
            GPIBPin::ATN => 9,
            GPIBPin::IFC => 23,
            GPIBPin::SRQ => 8,
            GPIBPin::REN => 27,
            GPIBPin::EOI => 25,
        }
    }
}
