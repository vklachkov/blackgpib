//! Raspberry Pi system-related tools.
//!
//! This file is taken from the `rppal` project (`https://github.com/golemparts/rppal`) under the MIT license,
//! and has been modified for the needs of the `blackgpib` project.

use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::result;

/// Identifiable Raspberry Pi models.
///
/// `Model` might be extended with additional variants in a minor or
/// patch revision, and must not be exhaustively matched against.
/// Instead, add a `_` catch-all arm to match future variants.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[non_exhaustive]
pub enum Model {
    RaspberryPiA,
    RaspberryPiAPlus,
    RaspberryPiBRev1,
    RaspberryPiBRev2,
    RaspberryPiBPlus,
    RaspberryPi2B,
    RaspberryPi3APlus,
    RaspberryPi3B,
    RaspberryPi3BPlus,
    RaspberryPi4B,
    RaspberryPi400,
    RaspberryPi5,
    RaspberryPi500,
    RaspberryPiComputeModule,
    RaspberryPiComputeModule3,
    RaspberryPiComputeModule3Plus,
    RaspberryPiComputeModule4,
    RaspberryPiComputeModule4S,
    RaspberryPiComputeModule5,
    RaspberryPiComputeModule5Lite,
    RaspberryPiZero,
    RaspberryPiZeroW,
    RaspberryPiZero2W,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Model::RaspberryPiA => write!(f, "Raspberry Pi A"),
            Model::RaspberryPiAPlus => write!(f, "Raspberry Pi A+"),
            Model::RaspberryPiBRev1 => write!(f, "Raspberry Pi B Rev 1"),
            Model::RaspberryPiBRev2 => write!(f, "Raspberry Pi B Rev 2"),
            Model::RaspberryPiBPlus => write!(f, "Raspberry Pi B+"),
            Model::RaspberryPi2B => write!(f, "Raspberry Pi 2 B"),
            Model::RaspberryPi3B => write!(f, "Raspberry Pi 3 B"),
            Model::RaspberryPi3BPlus => write!(f, "Raspberry Pi 3 B+"),
            Model::RaspberryPi3APlus => write!(f, "Raspberry Pi 3 A+"),
            Model::RaspberryPi4B => write!(f, "Raspberry Pi 4 B"),
            Model::RaspberryPi400 => write!(f, "Raspberry Pi 400"),
            Model::RaspberryPi5 => write!(f, "Raspberry Pi 5"),
            Model::RaspberryPi500 => write!(f, "Raspberry Pi 500"),
            Model::RaspberryPiComputeModule => write!(f, "Raspberry Pi Compute Module"),
            Model::RaspberryPiComputeModule3 => write!(f, "Raspberry Pi Compute Module 3"),
            Model::RaspberryPiComputeModule3Plus => write!(f, "Raspberry Pi Compute Module 3+"),
            Model::RaspberryPiComputeModule4 => write!(f, "Raspberry Pi Compute Module 4"),
            Model::RaspberryPiComputeModule4S => write!(f, "Raspberry Pi Compute Module 4S"),
            Model::RaspberryPiComputeModule5 => write!(f, "Raspberry Pi Compute Module 5"),
            Model::RaspberryPiComputeModule5Lite => write!(f, "Raspberry Pi Compute Module 5 Lite"),
            Model::RaspberryPiZero => write!(f, "Raspberry Pi Zero"),
            Model::RaspberryPiZeroW => write!(f, "Raspberry Pi Zero W"),
            Model::RaspberryPiZero2W => write!(f, "Raspberry Pi Zero 2 W"),
        }
    }
}

// GPIO registers on the RP1 have a different interface than the ones on earlier
// Broadcom SoCs
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum GpioInterface {
    Bcm,
    Rp1,
}

/// Identifiable Raspberry Pi SoCs.
///
/// `SoC` might be extended with additional variants in a minor or
/// patch revision, and must not be exhaustively matched against.
/// Instead, add a `_` catch-all arm to match future variants.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[non_exhaustive]
pub enum SoC {
    Bcm2835,
    Bcm2836,
    Bcm2837A1,
    Bcm2837B0,
    Bcm2711,
    Bcm2712,
}

impl fmt::Display for SoC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SoC::Bcm2835 => write!(f, "BCM2835"),
            SoC::Bcm2836 => write!(f, "BCM2836"),
            SoC::Bcm2837A1 => write!(f, "BCM2837A1"),
            SoC::Bcm2837B0 => write!(f, "BCM2837B0"),
            SoC::Bcm2711 => write!(f, "BCM2711"),
            SoC::Bcm2712 => write!(f, "BCM2712"),
        }
    }
}

// Identify Pi model based on /proc/cpuinfo
fn parse_proc_cpuinfo() -> io::Result<Model> {
    let proc_cpuinfo = BufReader::new(match File::open("/proc/cpuinfo") {
        Ok(file) => file,
        Err(_) => return Err(io::ErrorKind::Unsupported.into()),
    });

    let mut revision: String = String::new();
    for line in proc_cpuinfo.lines().map_while(result::Result::ok) {
        if let Some(line_value) = line.strip_prefix("Revision\t: ") {
            revision = String::from(line_value).to_lowercase();
        }
    }

    let model = if (revision.len() == 4) || (revision.len() == 8) {
        // Older revisions are 4 characters long, or 8 if they've been over-volted
        match &revision[revision.len() - 4..] {
            "0007" | "0008" | "0009" | "0015" => Model::RaspberryPiA,
            "beta" | "0002" | "0003" => Model::RaspberryPiBRev1,
            "0004" | "0005" | "0006" | "000d" | "000e" | "000f" => Model::RaspberryPiBRev2,
            "0012" => Model::RaspberryPiAPlus,
            "0010" | "0013" => Model::RaspberryPiBPlus,
            "0011" | "0014" => Model::RaspberryPiComputeModule,
            _ => return Err(io::ErrorKind::Unsupported.into()),
        }
    } else if revision.len() >= 6 {
        // Newer revisions consist of at least 6 characters

        // Compare just the type value for compatibility with future revisions
        let revision_type = match u64::from_str_radix(&revision, 16) {
            Ok(revision_type) => (revision_type >> 4) & 0xff,
            Err(_) => return Err(io::ErrorKind::Unsupported.into()),
        };

        match revision_type {
            0x00 => Model::RaspberryPiA,
            0x01 => Model::RaspberryPiBRev2,
            0x02 => Model::RaspberryPiAPlus,
            0x03 => Model::RaspberryPiBPlus,
            0x04 => Model::RaspberryPi2B,
            0x06 => Model::RaspberryPiComputeModule,
            0x08 => Model::RaspberryPi3B,
            0x09 => Model::RaspberryPiZero,
            0x0a => Model::RaspberryPiComputeModule3,
            0x0c => Model::RaspberryPiZeroW,
            0x0d => Model::RaspberryPi3BPlus,
            0x0e => Model::RaspberryPi3APlus,
            0x10 => Model::RaspberryPiComputeModule3Plus,
            0x11 => Model::RaspberryPi4B,
            0x12 => Model::RaspberryPiZero2W,
            0x13 => Model::RaspberryPi400,
            0x14 => Model::RaspberryPiComputeModule4,
            0x15 => Model::RaspberryPiComputeModule4S,
            0x17 => Model::RaspberryPi5,
            0x18 => Model::RaspberryPiComputeModule5,
            0x19 => Model::RaspberryPi500,
            0x1a => Model::RaspberryPiComputeModule5Lite,
            _ => return Err(io::ErrorKind::Unsupported.into()),
        }
    } else {
        return Err(io::ErrorKind::Unsupported.into());
    };

    Ok(model)
}

// Identify Pi model based on /sys/firmware/devicetree/base/compatible
fn parse_base_compatible() -> io::Result<Model> {
    let base_compatible = match fs::read_to_string("/sys/firmware/devicetree/base/compatible") {
        Ok(buffer) => buffer,
        Err(_) => return Err(io::ErrorKind::Unsupported.into()),
    };

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    for comp_id in base_compatible.split('\0') {
        let model = match comp_id {
            "raspberrypi,model-b-i2c0" => Model::RaspberryPiBRev1,
            "raspberrypi,model-b" => Model::RaspberryPiBRev1,
            "raspberrypi,model-a" => Model::RaspberryPiA,
            "raspberrypi,model-b-rev2" => Model::RaspberryPiBRev2,
            "raspberrypi,model-a-plus" => Model::RaspberryPiAPlus,
            "raspberrypi,model-b-plus" => Model::RaspberryPiBPlus,
            "raspberrypi,2-model-b" => Model::RaspberryPi2B,
            "raspberrypi,compute-module" => Model::RaspberryPiComputeModule,
            "raspberrypi,3-model-b" => Model::RaspberryPi3B,
            "raspberrypi,model-zero" => Model::RaspberryPiZero,
            "raspberrypi,3-compute-module" => Model::RaspberryPiComputeModule3,
            "raspberrypi,3-compute-module-plus" => Model::RaspberryPiComputeModule3Plus,
            "raspberrypi,model-zero-w" => Model::RaspberryPiZeroW,
            "raspberrypi,model-zero-2" => Model::RaspberryPiZero2W,
            "raspberrypi,model-zero-2-w" => Model::RaspberryPiZero2W,
            "raspberrypi,3-model-b-plus" => Model::RaspberryPi3BPlus,
            "raspberrypi,3-model-a-plus" => Model::RaspberryPi3APlus,
            "raspberrypi,4-model-b" => Model::RaspberryPi4B,
            "raspberrypi,400" => Model::RaspberryPi400,
            "raspberrypi,4-compute-module" => Model::RaspberryPiComputeModule4,
            "raspberrypi,4-compute-module-s" => Model::RaspberryPiComputeModule4S,
            "raspberrypi,5-model-b" => Model::RaspberryPi5,
            "raspberrypi,5-compute-module" => Model::RaspberryPiComputeModule5,
            "raspberrypi,500" => Model::RaspberryPi500,
            _ => continue,
        };

        return Ok(model);
    }

    Err(io::ErrorKind::Unsupported.into())
}

// Identify Pi model based on /sys/firmware/devicetree/base/model
fn parse_base_model() -> io::Result<Model> {
    let mut base_model = match fs::read_to_string("/sys/firmware/devicetree/base/model") {
        Ok(mut buffer) => {
            if let Some(idx) = buffer.find('\0') {
                buffer.truncate(idx);
            }

            buffer
        }
        Err(_) => return Err(io::ErrorKind::Unsupported.into()),
    };

    // Check if this is a Pi B rev 2 before we remove the revision part, assuming the
    // PCB Revision numbers on https://elinux.org/RPi_HardwareHistory are correct, and
    // the installed distro appends the revision to the model name.
    match &base_model[..] {
        "Raspberry Pi Model B Rev 2.0" => return Ok(Model::RaspberryPiBRev2),
        "Raspberry Pi Model B rev2 Rev 2.0" => return Ok(Model::RaspberryPiBRev2),
        "Raspberry Pi Zero 2 W Rev 1.0" => return Ok(Model::RaspberryPiZero2W),
        _ => (),
    }

    if let Some(idx) = base_model.find(" Rev ") {
        base_model.truncate(idx);
    }

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    let model = match &base_model[..] {
        "Raspberry Pi Model B (no P5)" => Model::RaspberryPiBRev1,
        "Raspberry Pi Model B" => Model::RaspberryPiBRev1,
        "Raspberry Pi Model A" => Model::RaspberryPiA,
        "Raspberry Pi Model B rev2" => Model::RaspberryPiBRev2,
        "Raspberry Pi Model A+" => Model::RaspberryPiAPlus,
        "Raspberry Pi Model A Plus" => Model::RaspberryPiAPlus,
        "Raspberry Pi Model B+" => Model::RaspberryPiBPlus,
        "Raspberry Pi Model B Plus" => Model::RaspberryPiBPlus,
        "Raspberry Pi 2 Model B" => Model::RaspberryPi2B,
        "Raspberry Pi Compute Module" => Model::RaspberryPiComputeModule,
        "Raspberry Pi 3 Model B" => Model::RaspberryPi3B,
        "Raspberry Pi Zero" => Model::RaspberryPiZero,
        "Raspberry Pi Compute Module 3" => Model::RaspberryPiComputeModule3,
        "Raspberry Pi Compute Module 3 Plus" => Model::RaspberryPiComputeModule3Plus,
        "Raspberry Pi Zero W" => Model::RaspberryPiZeroW,
        "Raspberry Pi Zero 2" => Model::RaspberryPiZero2W,
        "Raspberry Pi Zero 2 W" => Model::RaspberryPiZero2W,
        "Raspberry Pi 3 Model B+" => Model::RaspberryPi3BPlus,
        "Raspberry Pi 3 Model B Plus" => Model::RaspberryPi3BPlus,
        "Raspberry Pi 3 Model A Plus" => Model::RaspberryPi3APlus,
        "Raspberry Pi 4 Model B" => Model::RaspberryPi4B,
        "Raspberry Pi 400" => Model::RaspberryPi400,
        "Raspberry Pi Compute Module 4" => Model::RaspberryPiComputeModule4,
        "Raspberry Pi Compute Module 4S" => Model::RaspberryPiComputeModule4S,
        "Raspberry Pi 5 Model B" => Model::RaspberryPi5,
        "Raspberry Pi Compute Module 5" => Model::RaspberryPiComputeModule5,
        "Raspberry Pi Compute Module 5 Lite" => Model::RaspberryPiComputeModule5Lite,
        "Raspberry Pi 500" => Model::RaspberryPi500,
        _ => return Err(io::ErrorKind::Unsupported.into()),
    };

    Ok(model)
}

/// Retrieves Raspberry Pi device information.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct DeviceInfo {
    model: Model,
    soc: SoC,
    gpio_interface: GpioInterface,
}

impl DeviceInfo {
    /// Constructs a new `DeviceInfo`.
    ///
    /// `new` attempts to identify the Raspberry Pi's model and SoC based on
    /// the contents of `/proc/cpuinfo`, `/sys/firmware/devicetree/base/compatible`
    /// and `/sys/firmware/devicetree/base/model`.
    pub fn new() -> io::Result<DeviceInfo> {
        // Parse order from most-detailed to least-detailed info
        let model = parse_proc_cpuinfo().or_else(|_| parse_base_compatible().or_else(|_| parse_base_model()))?;

        // Set SoC and memory offsets based on model
        match model {
            Model::RaspberryPiA
            | Model::RaspberryPiAPlus
            | Model::RaspberryPiBRev1
            | Model::RaspberryPiBRev2
            | Model::RaspberryPiBPlus
            | Model::RaspberryPiComputeModule
            | Model::RaspberryPiZero
            | Model::RaspberryPiZeroW => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2835,
                gpio_interface: GpioInterface::Bcm,
            }),
            Model::RaspberryPi2B => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2836,
                gpio_interface: GpioInterface::Bcm,
            }),
            Model::RaspberryPi3B | Model::RaspberryPiComputeModule3 | Model::RaspberryPiZero2W => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2837A1,
                gpio_interface: GpioInterface::Bcm,
            }),
            Model::RaspberryPi3BPlus | Model::RaspberryPi3APlus | Model::RaspberryPiComputeModule3Plus => {
                Ok(DeviceInfo {
                    model,
                    soc: SoC::Bcm2837B0,
                    gpio_interface: GpioInterface::Bcm,
                })
            }
            Model::RaspberryPi4B
            | Model::RaspberryPi400
            | Model::RaspberryPiComputeModule4
            | Model::RaspberryPiComputeModule4S => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2711,
                gpio_interface: GpioInterface::Bcm,
            }),
            Model::RaspberryPi5
            | Model::RaspberryPi500
            | Model::RaspberryPiComputeModule5
            | Model::RaspberryPiComputeModule5Lite => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2712,
                gpio_interface: GpioInterface::Rp1,
            }),
        }
    }

    /// Returns the Raspberry Pi's model.
    pub fn model(&self) -> Model {
        self.model
    }

    /// Returns the Raspberry Pi's SoC.
    pub fn soc(&self) -> SoC {
        self.soc
    }

    /// Returns the GPIO interface type for this model.
    pub fn gpio_interface(&self) -> GpioInterface {
        self.gpio_interface
    }
}
