/// GPIB commands (messages with ATN) that a GRiD laptop can send to a device.
///
/// Commands are taken from the standard, section 2.13.7.1 Interface Messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
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

impl From<u8> for Command {
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

impl From<Command> for u8 {
    fn from(cmd: Command) -> u8 {
        match cmd {
            Command::DCL => 0b0001_0100,
            Command::SPE => 0b0001_1000,
            Command::SPD => 0b0001_1001,
            Command::UNL => 0b0011_1111,
            Command::UNT => 0b0101_1111,
            Command::MLA(val) => 0b0010_0000 | (val & 0b0001_1111),
            Command::MTA(val) => 0b0100_0000 | (val & 0b0001_1111),
            Command::Unsupported(val) => val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_variants() {
        assert_eq!(Command::from(0x14), Command::DCL);

        for i in 0..31 {
            assert_eq!(Command::from(0x20 | i), Command::MLA(i));
        }
        assert_eq!(Command::from(0x3f), Command::UNL);

        for i in 0..31 {
            assert_eq!(Command::from(0x40 | i), Command::MTA(i));
        }
        assert_eq!(Command::from(0x5f), Command::UNT);

        assert_eq!(Command::from(0x18), Command::SPE);
        assert_eq!(Command::from(0x19), Command::SPD);

        assert_eq!(Command::from(0x60), Command::Unsupported(0x60));
    }

    #[test]
    fn serialize_all_variants() {
        let byte: u8 = Command::DCL.into();
        assert_eq!(byte, 0x14);

        for i in 0..31 {
            let byte: u8 = Command::MLA(i).into();
            assert_eq!(byte, 0x20 | i);
        }
        let byte: u8 = Command::UNL.into();
        assert_eq!(byte, 0x3f);

        for i in 0..31 {
            let byte: u8 = Command::MTA(i).into();
            assert_eq!(byte, 0x40 | i);
        }
        let byte: u8 = Command::UNT.into();
        assert_eq!(byte, 0x5f);

        let byte: u8 = Command::SPE.into();
        assert_eq!(byte, 0x18);
        let byte: u8 = Command::SPD.into();
        assert_eq!(byte, 0x19);

        let byte: u8 = Command::Unsupported(0x60).into();
        assert_eq!(byte, 0x60);
    }

    #[test]
    fn round_trip_dcl() {
        let cmd = Command::DCL;
        let byte: u8 = cmd.into();
        let round_trip = Command::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_spe() {
        let cmd = Command::SPE;
        let byte: u8 = cmd.into();
        let round_trip = Command::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_spd() {
        let cmd = Command::SPD;
        let byte: u8 = cmd.into();
        let round_trip = Command::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_unl() {
        let cmd = Command::UNL;
        let byte: u8 = cmd.into();
        let round_trip = Command::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_unt() {
        let cmd = Command::UNT;
        let byte: u8 = cmd.into();
        let round_trip = Command::from(byte);
        assert_eq!(cmd, round_trip);
    }

    #[test]
    fn round_trip_mla() {
        for i in 0..31 {
            let cmd = Command::MLA(i);
            let byte: u8 = cmd.into();
            let round_trip = Command::from(byte);
            assert_eq!(cmd, round_trip);
        }
    }

    #[test]
    fn round_trip_mta() {
        for i in 0..31 {
            let cmd = Command::MTA(i);
            let byte: u8 = cmd.into();
            let round_trip = Command::from(byte);
            assert_eq!(cmd, round_trip);
        }
    }
}
