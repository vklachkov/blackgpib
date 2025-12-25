//! Valid pin list for the BlackGPiB board.

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum KnownPin {
    /// Transfer Enable/Talk Enable (SN75161 and 162).
    TE = 7,
    /// Direction Control (SN75161 and 162).
    DC = 11,
    /// System Control (SN75162 only).
    SC = 17,
    /// System Control (SN75161 and 162).
    PE = 21,

    /// Data Input/Output, bit 1.
    DIO1 = 5,
    /// Data Input/Output, bit 2.
    DIO2 = 6,
    /// Data Input/Output, bit 3.
    DIO3 = 12,
    /// Data Input/Output, bit 4.
    DIO4 = 13,
    /// Data Input/Output, bit 5.
    DIO5 = 19,
    /// Data Input/Output, bit 6.
    DIO6 = 16,
    /// Data Input/Output, bit 7.
    DIO7 = 26,
    /// Data Input/Output, bit 8.
    DIO8 = 20,

    /// Data Available.
    DAV = 10,
    /// Not Ready For Data.
    NRFD = 24,
    /// Not Data Accepted.
    NDAC = 22,

    /// Attention.
    ATN = 9,
    /// Interface Clear.
    IFC = 23,
    /// Service Request.
    SRQ = 8,
    /// Remote Enable.
    REN = 27,
    /// End Or Identify.
    EOI = 25,
}

impl KnownPin {
    pub const fn all() -> [Self; 20] {
        [
            KnownPin::TE,
            KnownPin::DC,
            KnownPin::SC,
            KnownPin::PE,
            KnownPin::DIO1,
            KnownPin::DIO2,
            KnownPin::DIO3,
            KnownPin::DIO4,
            KnownPin::DIO5,
            KnownPin::DIO6,
            KnownPin::DIO7,
            KnownPin::DIO8,
            KnownPin::DAV,
            KnownPin::NRFD,
            KnownPin::NDAC,
            KnownPin::ATN,
            KnownPin::IFC,
            KnownPin::SRQ,
            KnownPin::REN,
            KnownPin::EOI,
        ]
    }

    pub const fn data() -> [KnownPin; 8] {
        [
            KnownPin::DIO1,
            KnownPin::DIO2,
            KnownPin::DIO3,
            KnownPin::DIO4,
            KnownPin::DIO5,
            KnownPin::DIO6,
            KnownPin::DIO7,
            KnownPin::DIO8,
        ]
    }
}

const _: () = {
    let pins = KnownPin::all();

    let mut i = 0;
    while i < pins.len() {
        assert!((pins[i] as u8) < 32, "all pins must be in range 0..32");
        i += 1;
    }
};
