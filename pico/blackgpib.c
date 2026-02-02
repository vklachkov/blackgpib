#include "blackgpib.h"

#include "pico/stdlib.h"
#include "pico_fatfs/tf_card.h"
#include "pico_fatfs/fatfs/ff.h"

#include "hardware/gpio.h"

#include <stdio.h>

int first_chunk_test() {
    pico_fatfs_spi_config_t config = {
        spi0,
        CLK_SLOW_DEFAULT,
        CLK_FAST_DEFAULT,
        PIN_SD_CARD_MISO,
        PIN_SD_CARD_CS,
        PIN_SD_CARD_SCK,
        PIN_SD_CARD_MOSI,
        true
    };

    bool spi_configured = pico_fatfs_set_config(&config);
    if (!spi_configured) {
        printf("Failed to configure SPI\n");
        return 1;
    }

    FATFS fs;

    FRESULT ret = f_mount(&fs, "", 1);  // with force check
    if (ret != FR_OK) {
        printf("Failed to configure SD card\n");
        return 1;
    }

    switch (fs.fs_type) {
        case FS_FAT12:
            printf("FS type is FAT12\n");
            break;
        case FS_FAT16:
            printf("FS type is FAT16\n");
            break;
        case FS_FAT32:
            printf("FS type is FAT32\n");
            break;
        case FS_EXFAT:
            printf("FS type is ExFAT\n");
            break;
        default:
            printf("FS type is unknown\n");
            break;
    }

    printf("Card size: %0.2f GB\n", fs.csize * fs.n_fatent * 512E-9);

    FIL fp;

    ret = f_open(&fp, "CCOS310.IMG", FA_READ | FA_WRITE | FA_OPEN_EXISTING);
    if (ret != FR_OK) {
        printf("Failed to open data.img\n");
        return 1;
    }

    uint8_t buffer[512];
    size_t read;

    ret = f_read(&fp, &buffer, 512, &read);
    if (ret != FR_OK) {
        printf("Failed to read chunk from data.img\n");
        return 1;
    }

    printf("First chunk header: %s\n", &buffer);
}

int main() {
    stdio_init_all();

    first_chunk_test();

    uint pin = PIN_GPIB_DIO7;

    gpio_init(pin);
    gpio_set_dir(pin, GPIO_OUT);
    gpio_put(pin, 1);

    while (true) {
        gpio_put(pin, 1);

        sleep_ms(3000);

        gpio_put(pin, 0);

        sleep_ms(3000);
    }
}
