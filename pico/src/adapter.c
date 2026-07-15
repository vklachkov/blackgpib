#include "adapter.h"

#include "gpio.h"
#include "usb_cdc.h"
#include "watchdog.h"

#include "hardware/gpio.h"
#include "pico/multicore.h"
#include "pico/stdlib.h"

#include <stdio.h>
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

static void adapter_version(void) {
  printf("BLACKGPIB-" PICO_PROGRAM_VERSION_STRING "\r\n");
}

static void adapter_disk_status(const char* device) {
  (void)device;
}

static void adapter_read_sector(const char* device, const char* sector) {
  (void)device;
  (void)sector;
}

static void adapter_gpib_reset(void) {
}

static void adapter_handle_command(const usb_cdc_command_t* command) {
  if (command->form == USB_CDC_EXECUTE && strcmp(command->name, "VERSION") == 0) {
    adapter_version();
  } else if (command->form == USB_CDC_SET && command->argc == 1 &&
             strcmp(command->name, "STATUS") == 0) {
    adapter_disk_status(command->args[0]);
  } else if (command->form == USB_CDC_SET && command->argc == 2 &&
             strcmp(command->name, "READ") == 0) {
    adapter_read_sector(command->args[0], command->args[1]);
  } else if (command->form == USB_CDC_EXECUTE &&
             strcmp(command->name, "GPIB_RESET") == 0) {
    adapter_gpib_reset();
  } else {
    printf("ERROR\r\n");
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

    adapter_handle_command(&command);
  }
}
