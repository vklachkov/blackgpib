/// GPIB commands (messages with ATN) that a GRiD laptop can send
/// to a hard drive or floppy drive.
/// 
/// Commands are taken from the standard, section 2.13.7.1 Interface Messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPIBCommand {
    /// Device Clear.
    DCL,

    /// Selected Device Clear.
    SDC,

    /// Serial Poll Enable.
    SPE,

    /// Serial Poll Disable.
    SPD,

    /// My Listen Address.
    MLA(u8),

    /// Unlisten.
    UNL,

    /// My Talking Address.
    MTA(u8),

    /// Untalk.
    UNT,

    /// Unsupported command.
    Unsupported(u8),
}

impl From<u8> for GPIBCommand {
    fn from(value: u8) -> Self {
        if value == 0b0001_0100 {
            Self::DCL
        } else if value == 0b0000_0100 {
            Self::SDC
        } else if value == 0b0001_1000 {
            Self::SPE
        } else if value == 0b0001_1001 {
            Self::SPD
        } else if value == 0b0011_1111 {
            Self::UNL
        } else if value == 0b0101_1111 {
            Self::UNT
        } else if (value & 0b0110_0000) == 0b0010_0000 {
            Self::MLA(value & 0b0001_1111)
        } else if (value & 0b0110_0000) == 0b0100_0000 {
            Self::MTA(value & 0b0001_1111)
        } else {
            Self::Unsupported(value)
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn parse_all_variants() {
        assert_eq!(GPIBCommand::from(0x14), GPIBCommand::DCL);
        assert_eq!(GPIBCommand::from(0x04), GPIBCommand::SDC);

        for i in 0..31 {
            assert_eq!(GPIBCommand::from(0x20 | i), GPIBCommand::MLA(i));
        }
        assert_eq!(GPIBCommand::from(0x3f), GPIBCommand::UNL);

        for i in 0..31 {
            assert_eq!(GPIBCommand::from(0x40 | i), GPIBCommand::MTA(i));
        }
        assert_eq!(GPIBCommand::from(0x5f), GPIBCommand::UNT);

        assert_eq!(GPIBCommand::from(0x18), GPIBCommand::SPE);
        assert_eq!(GPIBCommand::from(0x19), GPIBCommand::SPD);

        assert_eq!(GPIBCommand::from(0x60), GPIBCommand::Unsupported(0x60));
    }
}
