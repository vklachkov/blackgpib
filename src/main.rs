mod devices;
mod gpib_command;
mod gpib_gpio;
mod gpib_pinout;
mod listener;
mod logger;
mod talker;
mod utils;

use std::fs;

use crate::{devices::DeviceManager, utils::configure_scheduler};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    logger::setup();

    info!("BlackGPiB v{VERSION} started");

    debug!("Reset all pins to Z-State...");
    gpib_gpio::reset_all();

    debug!("Setup scheduler...");
    configure_scheduler();

    let mut devman = DeviceManager::new();

    devman.insert_image(0, fs::read("2101_6ext_fixed").unwrap(), 0x121, 0x120);
    devman.insert_image(1, fs::read("disk1").unwrap(), 0x121, 0x120);
    // devman.insert_image(2, fs::read("XECUT_BOOT.IMG").unwrap(), 0x121, 0x120);
    // devman.insert_image(3, fs::read("GRIDOS.IMG").unwrap(), 0x121, 0x120);
    // devman.insert_image(4, fs::read("disk1").unwrap(), 0x121, 0x120);

    devman.start();
}
