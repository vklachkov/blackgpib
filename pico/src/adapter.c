#include "adapter.h"

#include "blackgpib.h"
#include "gpio.h"
#include "usb_cdc.h"
#include "watchdog.h"

#include "hardware/gpio.h"
#include "pico/multicore.h"
#include "pico/stdlib.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

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

static bool adapter_parse_uint32(const char* text, uint32_t* value) {
  char* end;
  unsigned long number = strtoul(text, &end, 10);

  if (*text == '\0' || *end != '\0' || number > UINT32_MAX) return false;
  *value = (uint32_t)number;
  return true;
}

static void adapter_print_hex(const uint8_t* buffer, size_t size) {
  for (size_t i = 0; i < size; i++) {
    printf(i == 0 ? "%02X" : " %02X", buffer[i]);
  }
  printf("\r\n");
}

static void adapter_version(void) {
  printf("BLACKGPIB-" PICO_PROGRAM_VERSION_STRING "\r\n");
}

static void adapter_disk_status(const char* device) {
  (void)device;
}

static void adapter_read_sector(const char* device, const char* sector) {
  uint32_t device_number;
  uint32_t sector_number;

  if (!adapter_parse_uint32(device, &device_number) || device_number >= MAX_DEVICES ||
      !adapter_parse_uint32(sector, &sector_number) || sector_number > 0xFFFF) {
    printf("ERROR\r\n");
    return;
  }

  static const uint8_t buffer[] = {0xA1, 0xA3, 0xB3, 0xBD, 0x12, 0x34, 0xF3};
  adapter_print_hex(buffer, sizeof(buffer));
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
