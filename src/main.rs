mod devices;
mod gpib_command;
mod gpib_gpio;
mod gpib_pinout;
mod listener;
mod logger;
mod talker;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    //

    logger::setup();

    info!("BlackGPiB v{VERSION} started");

    debug!("Reset all pins to Z-State...");
    // gpib_gpio::reset_all();

    //
}
