#include "adapter.h"

#include "blackgpib.h"
#include "disk_protocol.h"
#include "gpio.h"
#include "gpib.h"
#include "usb_cdc.h"
#include "watchdog.h"

#include "hardware/gpio.h"
#include "pico/multicore.h"
#include "pico/stdlib.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define ADAPTER_GPIB_TIMEOUT_MS  4000

#define GPIB_DCL  0x14
#define GPIB_SPE  0x18
#define GPIB_SPD  0x19
#define GPIB_UNL  0x3F
#define GPIB_UNT  0x5F

#define ADAPTER_SECTOR_SIZE  512

static bool adapter_gpib_wait(uint pin, bool value, absolute_time_t until) {
  while (gpio_get(pin) != value) {
    if (time_reached(until)) return false;
    tight_loop_contents();
  }
  return true;
}

static bool adapter_gpib_send_command(uint8_t command, absolute_time_t until) {
  gpio_put(PIN_GPIB_ATN, false);
  sleep_us(5);

  if (!adapter_gpib_wait(PIN_GPIB_NDAC, false, until)) goto failed;

  gpib_write_byte(command);
  sleep_us(10);
  gpio_put(PIN_GPIB_DAV, false);
  sleep_us(5);

  if (!adapter_gpib_wait(PIN_GPIB_NDAC, true, until)) goto failed;

  gpio_put(PIN_GPIB_DAV, true);
  sleep_us(5);
  return true;

failed:
  gpio_put(PIN_GPIB_DAV, true);
  gpio_put(PIN_GPIB_ATN, true);
  return false;
}

static bool adapter_gpib_send_byte(uint8_t byte, bool eoi, absolute_time_t until) {
  gpio_put(PIN_GPIB_EOI, !eoi);
  gpib_write_byte(byte);
  sleep_us(10);

  if (!adapter_gpib_wait(PIN_GPIB_NDAC, false, until) ||
      !adapter_gpib_wait(PIN_GPIB_NRFD, true, until)) {
    gpio_put(PIN_GPIB_DAV, true);
    gpio_put(PIN_GPIB_EOI, true);
    return false;
  }

  gpio_put(PIN_GPIB_DAV, false);
  if (!adapter_gpib_wait(PIN_GPIB_NDAC, true, until)) {
    gpio_put(PIN_GPIB_DAV, true);
    gpio_put(PIN_GPIB_EOI, true);
    return false;
  }

  gpio_put(PIN_GPIB_DAV, true);
  gpio_put(PIN_GPIB_EOI, true);
  return true;
}

static bool adapter_gpib_send_bytes(const uint8_t* buffer, size_t size, absolute_time_t until) {
  gpio_put(PIN_GPIB_ATN, true);

  for (size_t i = 0; i < size; i++) {
    if (!adapter_gpib_send_byte(buffer[i], i == size - 1, until)) return false;
    sleep_us(20);
  }
  return true;
}

static bool adapter_gpib_send_request(uint8_t device, uint8_t code, uint32_t sector,
                                      uint16_t data_size, absolute_time_t until) {
  const uint8_t request[REQUEST_LEN] = {
    code, 0, 0,
    (uint8_t)sector, (uint8_t)(sector >> 8),
    (uint8_t)(sector >> 16), (uint8_t)(sector >> 24),
    (uint8_t)data_size, (uint8_t)(data_size >> 8), 0,
  };

  gpib_configure_talker();
  bool sent = adapter_gpib_send_command(0x20 | device, until) &&
              adapter_gpib_send_bytes(request, sizeof(request), until);
  bool unlistened = adapter_gpib_send_command(GPIB_UNL, until);
  return sent && unlistened;
}

static bool adapter_gpib_read_byte(uint8_t* byte, bool* eoi, absolute_time_t until) {
  gpio_put(PIN_GPIB_NDAC, false);
  if (!adapter_gpib_wait(PIN_GPIB_DAV, false, until)) return false;

  *byte = gpib_read_byte();
  if (eoi != NULL) *eoi = !gpio_get(PIN_GPIB_EOI);
  return true;
}

static bool adapter_gpib_end_handshake(absolute_time_t until) {
  gpio_put(PIN_GPIB_NRFD, false);
  gpio_put(PIN_GPIB_NDAC, true);

  bool complete = adapter_gpib_wait(PIN_GPIB_DAV, true, until);

  gpio_put(PIN_GPIB_NDAC, false);
  gpio_put(PIN_GPIB_NRFD, true);
  return complete;
}

static bool adapter_gpib_read_response(uint8_t device, uint8_t* buffer, size_t size,
                                       size_t* length, absolute_time_t until) {
  gpib_configure_talker();
  if (!adapter_gpib_send_command(0x40 | device, until)) return false;

  gpib_configure_listener();
  *length = 0;
  bool received = false;
  while (true) {
    uint8_t byte;
    bool eoi;

    if (!adapter_gpib_read_byte(&byte, &eoi, until) ||
        !adapter_gpib_end_handshake(until) || *length == size) break;

    buffer[(*length)++] = byte;
    if (eoi) {
      received = true;
      break;
    }
  }

  gpib_configure_talker();
  bool untalked = adapter_gpib_send_command(GPIB_UNT, until);
  return received && untalked;
}

static bool adapter_gpib_wait_ready(uint8_t device, absolute_time_t until) {
  uint8_t response;

  if (!adapter_gpib_wait(PIN_GPIB_SRQ, false, until)) return false;

  gpib_configure_talker();
  bool selected = adapter_gpib_send_command(GPIB_SPE, until) &&
                  adapter_gpib_send_command(0x40 | device, until);
  bool ready = false;

  if (selected) {
    gpib_configure_listener();
    ready = adapter_gpib_read_byte(&response, NULL, until) &&
            adapter_gpib_end_handshake(until) && response == 0x4F;
  }

  gpib_configure_talker();
  bool stopped = adapter_gpib_send_command(GPIB_SPD, until) &&
                 adapter_gpib_send_command(GPIB_UNT, until);
  return selected && ready && stopped;
}

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
  uint8_t buffer[STATUS_LEN];
  uint32_t device_number;
  size_t length;
  absolute_time_t until = make_timeout_time_ms(ADAPTER_GPIB_TIMEOUT_MS);

  if (!adapter_parse_uint32(device, &device_number) || device_number >= MAX_DEVICES ||
      !adapter_gpib_send_request((uint8_t)device_number, DISK_REQ_GET_STATUS, 0, 54, until) ||
      !adapter_gpib_read_response((uint8_t)device_number, buffer, sizeof(buffer), &length, until)) {
    gpib_configure_listener();
    printf("ERROR\r\n");
    return;
  }
  adapter_print_hex(buffer, length);
}

static void adapter_read_sector(const char* device, const char* sector) {
  uint32_t device_number;
  uint32_t sector_number;
  uint8_t buffer[ADAPTER_SECTOR_SIZE];
  size_t length;
  absolute_time_t until = make_timeout_time_ms(ADAPTER_GPIB_TIMEOUT_MS);

  if (!adapter_parse_uint32(device, &device_number) || device_number >= MAX_DEVICES ||
      !adapter_parse_uint32(sector, &sector_number) || sector_number > 0xFFFF) {
    printf("ERROR\r\n");
    return;
  }

  if (!adapter_gpib_send_request((uint8_t)device_number, DISK_REQ_READ, sector_number,
                                 ADAPTER_SECTOR_SIZE, until) ||
      !adapter_gpib_wait_ready((uint8_t)device_number, until) ||
      !adapter_gpib_read_response((uint8_t)device_number, buffer, sizeof(buffer), &length, until)) {
    gpib_configure_listener();
    printf("ERROR\r\n");
    return;
  }
  adapter_print_hex(buffer, length);
}

static void adapter_gpib_reset(void) {
  absolute_time_t until = make_timeout_time_ms(ADAPTER_GPIB_TIMEOUT_MS);

  gpib_configure_talker();
  if (!adapter_gpib_send_command(GPIB_DCL, until)) printf("ERROR\r\n");
  gpib_configure_listener();
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
  gpib_preconfigure_pins();

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
