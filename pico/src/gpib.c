#include "gpib.h"
#include "gpio.h"

#include "pico/stdlib.h"

#include <stdio.h>

gpib_cmd_t gpib_parse_cmd(uint8_t value) {
  if (value == 0b00010100)
    return (gpib_cmd_t){ value, GPIB_CMD_DCL, 0 };
  else if (value == 0b00011000)
    return (gpib_cmd_t){ value, GPIB_CMD_SPE, 0 };
  else if (value == 0b00011001)
    return (gpib_cmd_t){ value, GPIB_CMD_SPD, 0 };
  else if (value == 0b00111111)
    return (gpib_cmd_t){ value, GPIB_CMD_UNL, 0 };
  else if (value == 0b01011111)
    return (gpib_cmd_t){ value, GPIB_CMD_UNT, 0 };
  else if ((value & 0b01100000) == 0b00100000)
    return (gpib_cmd_t){ value, GPIB_CMD_MLA, (uint8_t)(value & 0b00011111) };
  else if ((value & 0b01100000) == 0b01000000)
    return (gpib_cmd_t){ value, GPIB_CMD_MTA, (uint8_t)(value & 0b00011111) };
  else
    return (gpib_cmd_t){ value, GPIB_CMD_UNKNOWN, 0 };
}

void gpib_cmd_debug(const gpib_cmd_t* cmd) {
  switch (cmd->type) {
    case GPIB_CMD_DCL:
      printf("gpib: DCL\n");
      break;

    case GPIB_CMD_SPE:
      printf("gpib: SPE\n");
      break;

    case GPIB_CMD_SPD:
      printf("gpib: SPD\n");
      break;

    case GPIB_CMD_MLA:
      printf("gpib: MLA(%d)\n", cmd->addr);
      break;
    
    case GPIB_CMD_UNL:
      printf("gpib: UNL\n");
      break;

    case GPIB_CMD_MTA:
      printf("gpib: MTA(%d)\n", cmd->addr);
      break;

    case GPIB_CMD_UNT:
      printf("gpib: UNT\n");
      break;

    default:
      printf("gpib: unknown command 0x%02x\n", cmd->raw);
      break;
  }
}

void gpib_configure_control_pins(void) {
  configure_output(PIN_GPIB_DC);
  configure_output(PIN_GPIB_TE);
}

void gpib_configure_listener(void) {  
  configure_input(PIN_GPIB_ATN);
  configure_output(PIN_GPIB_SRQ);
  configure_input(PIN_GPIB_REN);
  configure_input(PIN_GPIB_IFC);
  configure_input(PIN_GPIB_EOI);
  configure_input(PIN_GPIB_DAV);

  configure_output(PIN_GPIB_NDAC);
  configure_output(PIN_GPIB_NRFD);

  configure_input(PIN_GPIB_DIO1);
  configure_input(PIN_GPIB_DIO2);
  configure_input(PIN_GPIB_DIO3);
  configure_input(PIN_GPIB_DIO4);
  configure_input(PIN_GPIB_DIO5);
  configure_input(PIN_GPIB_DIO6);
  configure_input(PIN_GPIB_DIO7);
  configure_input(PIN_GPIB_DIO8);

  gpio_put(PIN_GPIB_DC, true);
  gpio_put(PIN_GPIB_TE, false);
}

uint8_t gpib_read_byte(void) {
  uint8_t result = 0;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO1)) << 0;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO2)) << 1;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO3)) << 2;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO4)) << 3;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO5)) << 4;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO6)) << 5;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO7)) << 6;
  result |= (uint8_t)(!gpio_get(PIN_GPIB_DIO8)) << 7;
  return result;
}

gpib_cmd_t gpib_start_command_handshake(void) {
  while (true) {
    if (gpio_get(PIN_GPIB_ATN)) {
      gpio_put(PIN_GPIB_NDAC, true);
      continue;
    }

    gpio_put(PIN_GPIB_NDAC, false);

    if (gpio_get(PIN_GPIB_DAV)) {
      continue;
    }

    gpio_put(PIN_GPIB_NRFD, false);

    uint8_t byte = gpib_read_byte();
    return gpib_parse_cmd(byte);
  }
}

void gpib_unexpected_byte(void) {
  gpio_put(PIN_GPIB_NDAC, true);

  while (!gpio_get(PIN_GPIB_DAV)) {}

  gpio_put(PIN_GPIB_NRFD, true);
}

void gpib_end_handshake(void) {
  gpio_put(PIN_GPIB_NRFD, false);
  gpio_put(PIN_GPIB_NDAC, true);

  while (!gpio_get(PIN_GPIB_DAV)) {}

  gpio_put(PIN_GPIB_NDAC, false);
  gpio_put(PIN_GPIB_NRFD, true);
}

gpib_byte_t gpib_start_data_handshake(void) {
  gpio_put(PIN_GPIB_NDAC, false);

  while (gpio_get(PIN_GPIB_DAV)) {}

  uint8_t value = gpib_read_byte();
  bool atn = gpio_get(PIN_GPIB_ATN);
  bool eoi = gpio_get(PIN_GPIB_EOI);

  return (gpib_byte_t){ value, atn, eoi };
}

void gpib_configure_talker(void) {
  configure_output(PIN_GPIB_ATN);
  configure_input(PIN_GPIB_SRQ);
  configure_output(PIN_GPIB_REN);
  configure_output(PIN_GPIB_IFC);
  configure_output(PIN_GPIB_EOI);
  configure_output(PIN_GPIB_DAV);

  configure_input(PIN_GPIB_NDAC);
  configure_input(PIN_GPIB_NRFD);

  configure_output(PIN_GPIB_DIO1);
  configure_output(PIN_GPIB_DIO2);
  configure_output(PIN_GPIB_DIO3);
  configure_output(PIN_GPIB_DIO4);
  configure_output(PIN_GPIB_DIO5);
  configure_output(PIN_GPIB_DIO6);
  configure_output(PIN_GPIB_DIO7);
  configure_output(PIN_GPIB_DIO8);

  gpio_put(PIN_GPIB_DC, false);
  gpio_put(PIN_GPIB_TE, true);
}

void gpib_write_byte(uint8_t byte) {
  const uint8_t pins[8] = { PIN_GPIB_DIO1, PIN_GPIB_DIO2, PIN_GPIB_DIO3, PIN_GPIB_DIO4, PIN_GPIB_DIO5, PIN_GPIB_DIO6, PIN_GPIB_DIO7, PIN_GPIB_DIO8 };

  for (size_t i = 0; i < 8; i++) {
    gpio_put(pins[i], !(byte >> i & 0b1));
  }
}

void gpib_send_byte(uint8_t byte, bool eoi) {
  gpio_put(PIN_GPIB_EOI, !eoi);
  gpib_write_byte(byte);

  sleep_us(10);

  while (gpio_get(PIN_GPIB_NDAC) || !gpio_get(PIN_GPIB_NRFD)) {}

  // Now we can signal that data is valid.
  gpio_put(PIN_GPIB_DAV, false);

  // Wait until the laptop signals successful data read.
  while (!gpio_get(PIN_GPIB_NDAC)) {}

  // Signal that data is no longer valid.
  gpio_put(PIN_GPIB_DAV, true);

  gpio_put(PIN_GPIB_EOI, true);
}

void gpib_send_serial_poll_response(uint8_t byte) {
  gpib_send_byte(byte, false);
}

void gpib_send_bytes(const uint8_t *data, size_t size) {
  gpio_put(PIN_GPIB_ATN, true);

  for (size_t i = 0; i < size; i++) {
    gpib_send_byte(data[i], i == size - 1);
    sleep_us(20);
  }
}
