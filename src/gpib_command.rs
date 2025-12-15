/// GPIB commands (messages with ATN) that a GRiD laptop can send to a device.
///
/// Commands are taken from the standard, section 2.13.7.1 Interface Messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPIBCommand {
    /// Device Clear.
    DCL,

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

impl From<GPIBCommand> for u8 {
    fn from(cmd: GPIBCommand) -> u8 {
        match cmd {
            GPIBCommand::DCL => 0b0001_0100,
            GPIBCommand::SPE => 0b0001_1000,
            GPIBCommand::SPD => 0b0001_1001,
            GPIBCommand::UNL => 0b0011_1111,
            GPIBCommand::UNT => 0b0101_1111,
            GPIBCommand::MLA(val) => 0b0010_0000 | (val & 0b0001_1111),
            GPIBCommand::MTA(val) => 0b0100_0000 | (val & 0b0001_1111),
            GPIBCommand::Unsupported(val) => val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_variants() {
        assert_eq!(GPIBCommand::from(0x14), GPIBCommand::DCL);

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

    #[test]
    fn serialize_all_variants() {
        let byte: u8 = GPIBCommand::DCL.into();
        assert_eq!(byte, 0x14);

        for i in 0..31 {
            let byte: u8 = GPIBCommand::MLA(i).into();
            assert_eq!(byte, 0x20 | i);
        }
        let byte: u8 = GPIBCommand::UNL.into();
        assert_eq!(byte, 0x3f);

        for i in 0..31 {
            let byte: u8 = GPIBCommand::MTA(i).into();
            assert_eq!(byte, 0x40 | i);
        }
        let byte: u8 = GPIBCommand::UNT.into();
        assert_eq!(byte, 0x5f);

        let byte: u8 = GPIBCommand::SPE.into();
        assert_eq!(byte, 0x18);
        let byte: u8 = GPIBCommand::SPD.into();
        assert_eq!(byte, 0x19);

        let byte: u8 = GPIBCommand::Unsupported(0x60).into();
        assert_eq!(byte, 0x60);
    }

    #[test]
    fn round_trip_dcl() {
        let cmd = GPIBCommand::DCL;
        let byte: u8 = cmd.into();
        let round_trip = GPIBCommand::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_spe() {
        let cmd = GPIBCommand::SPE;
        let byte: u8 = cmd.into();
        let round_trip = GPIBCommand::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_spd() {
        let cmd = GPIBCommand::SPD;
        let byte: u8 = cmd.into();
        let round_trip = GPIBCommand::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_unl() {
        let cmd = GPIBCommand::UNL;
        let byte: u8 = cmd.into();
        let round_trip = GPIBCommand::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_unt() {
        let cmd = GPIBCommand::UNT;
        let byte: u8 = cmd.into();
        let round_trip = GPIBCommand::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_mla() {
        for i in 0..31 {
            let cmd = GPIBCommand::MLA(i);
            let byte: u8 = cmd.into();
            let round_trip = GPIBCommand::from(byte);
            assert_eq!(cmd, round_trip);
        }
    }

    #[test]
    fn round_trip_mta() {
        for i in 0..31 {
            let cmd = GPIBCommand::MTA(i);
            let byte: u8 = cmd.into();
            let round_trip = GPIBCommand::from(byte);
            assert_eq!(cmd, round_trip);
        }
    }
}
