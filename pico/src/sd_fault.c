#include "sd_fault.h"

#include "gpio.h"

#include "pico/stdlib.h"
#include "hardware/watchdog.h"
#include "hardware/gpio.h"

#include "pico_fatfs/fatfs/ff.h"


__attribute__((noreturn))
void sd_card_fault(void) {
    FATFS fs;

    // Unmount previous mount.
    f_unmount("");

    while (true) {
        // Try to mount sd card...
        if (f_mount(&fs, "", 1) == FR_OK)
            break;

        // Blink on fail.
        gpio_xor_mask(1 << PIN_LED);

        sleep_ms(1000);
    }

    // SD card is reinserted, we give the user an indication.
    gpio_put(PIN_LED, 0);
    for (int i = 0; i < 8; i++) {
      gpio_xor_mask(1 << PIN_LED);
      sleep_ms(100);
    }
    gpio_put(PIN_LED, 0);

    // And go to reboot to reinitialize device.
    watchdog_reboot(0, 0, 0);
}
