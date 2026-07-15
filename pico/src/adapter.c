#include "adapter.h"

#include "gpio.h"
#include "usb_cdc.h"
#include "watchdog.h"

#include "hardware/gpio.h"
#include "pico/multicore.h"
#include "pico/stdlib.h"

#include <string.h>

void adapter_wait_connect(void) {
  while (true) {
    usb_cdc_command_t command;
    usb_cdc_read_command(&command);

    if (command.form == USB_CDC_EXECUTE &&
        strcmp(command.name, "CONNECT") == 0) {
      wd_reboot_to_adapter();
    }
  }
}

static void blink_adapter(void) {
  int level = 0;
  int step = 1;

  while (true) {
    gpio_put(PIN_LED, level != 0);
    sleep_us((uint32_t)level * 20);
    gpio_put(PIN_LED, 0);
    sleep_us((uint32_t)(255 - level) * 20);

    level += step;
    if (level == 0 || level == 255) step = -step;
  }
}

void adapter_run(void) {
  gpio_init(PIN_LED);
  gpio_set_dir(PIN_LED, GPIO_OUT);

  multicore_launch_core1(blink_adapter);

  while (true) {
    usb_cdc_command_t command;
    usb_cdc_read_command(&command);

    if (command.form == USB_CDC_EXECUTE &&
        strcmp(command.name, "DISCONNECT") == 0) {
      wd_reboot_to_emulator();
    }
  }
}
