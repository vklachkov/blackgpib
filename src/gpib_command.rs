/// GPiB commands (messages with ATN) described in
/// 2.13.7.1 Interface Messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPIBCommand {
    /// Device Clear.
    DCL,

    /// My Listen Address.
    MLA(u8),

    /// My Talking Address.
    MTA(u8),

    /// Parallel Poll Enable.
    PPE { sense: u8, line: u8 },

    /// Parallel Poll Disable.
    PPD,

    /// Parallel Poll Unconfigure.
    PPU,

    /// Selected Device Clear.
    SDC,

    /// Serial Poll Disable.
    SPD,

    /// Serial Poll Enable.
    SPE,

    /// Unlisten.
    UNL,

    /// Untalk.
    UNT,

    /// Unsupported command.
    Unsupported(u8),
}

impl From<u8> for GPIBCommand {
    fn from(value: u8) -> Self {
        if (value & 0b0111_1111) == 0b0001_0100 {
            Self::DCL
        } else if (value & 0b0111_0000) == 0b0111_0000 {
            Self::PPD
        } else if (value & 0b0111_1111) == 0b0001_0101 {
            Self::PPU
        } else if (value & 0b0111_1111) == 0b0000_0100 {
            Self::SDC
        } else if (value & 0b0111_1111) == 0b0001_1001 {
            Self::SPD
        } else if (value & 0b0111_1111) == 0b0001_1000 {
            Self::SPE
        } else if (value & 0b0111_1111) == 0b0011_1111 {
            Self::UNT
        } else if (value & 0b0111_1111) == 0b0101_1111 {
            Self::UNL
        } else if (value & 0b0110_0000) == 0b0010_0000 {
            Self::MLA(value & 0b0001_1111)
        } else if (value & 0b0110_0000) == 0b0100_0000 {
            Self::MTA(value & 0b0001_1111)
        } else if (value & 0b0111_0000) == 0b0110_0000 {
            Self::PPE {
                sense: (value >> 3) & 0b01,
                line: value & 0b0000_0111,
            }
        } else {
            Self::Unsupported(value)
        }
    }
}
