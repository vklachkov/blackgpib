# BlackGPIB

BlackGPIB is hardware that emulates GPIB disk drives for the GRiD Compass. It is based on the Raspberry Pi RP2040.

![Pico Rev 1](photo/pico-rev1.jpg)

## Hardware

Hardware schematics and design files are located in the `hardware` folder. Production files are available in the [latest release](github.com/vklachkov/blackgpib/releases) on GitHub.

We recommend using the latest `pico` version. The list of components and known issues is described in the `README.md` file located in the `hardware/pico` directory.

## Firmware

The firmware source code is located in the `pico` folder.

The latest stable prebuilt firmware in UF2 format is available in the [latest release](https://github.com/vklachkov/blackgpib/releases) on GitHub.

### Flashing prebuilt firmware

To flash the firmware, hold the **BOOTSEL** button while connecting the Raspberry Pi Pico to your computer. The Pico will appear as a removable drive.

Copy the `blackgpib.uf2` file to this drive. The Pico will reboot automatically after the file is copied.

### Building firmware from source

To build the firmware yourself, install the Pico SDK first. Follow the instructions on the official Raspberry Pi website:

https://www.raspberrypi.com/documentation/microcontrollers/c_sdk.html

Then follow these steps.

1. Clone the repository:

```bash
git clone --depth 1 --recursive https://github.com/vklachkov/blackgpib
```

2. Go to the sources directory:

```bash
cd blackgpib/pico
```

3. Configure the project for your board:

```bash
cmake -S . -B build -DPICO_BOARD=pico
```

4. Build:

```bash
cmake --build build -j
```

5. Flash the firmware:

    Hold the **BOOTSEL** button while connecting the Raspberry Pi Pico to your computer. Then copy the `blackgpib.uf2` file from the `build` folder to the mounted Pico drive.

## License

## License

This project is released under the [MIT License](LICENSE).

You are free to use, modify, and share it. I will be happy if BlackGPIB helps you bring your GRiD Compass back to life and use it again.