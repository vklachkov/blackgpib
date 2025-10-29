#[derive(Clone, Copy)]
enum Pins {
    /// Data Input/Output, bit 0.
    DIO0,
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
