/// Checks if the byte is an My Listen Address
/// command for given address.
pub const fn is_mla(byte: u8, address: u8) -> bool {
    (byte & 0b0111_1111) == (0b0010_0000 | address)
}

/// Checks if the byte is an Unlisten
/// command for given address.
pub const fn is_unl(byte: u8) -> bool {
    (byte & 0b0111_1111) == 0b0011_1111
}

/// Checks if the byte is an My Talk Address
/// command for given address.
pub const fn is_mta(byte: u8, address: u8) -> bool {
    (byte & 0b0111_1111) == (0b0100_0000 | address)
}

/// Checks if the byte is an Device CLear command.
pub const fn is_dcl(byte: u8) -> bool {
    (byte & 0b0111_1111) == (0b0001_0100)
}
