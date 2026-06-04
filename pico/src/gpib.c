#include "gpib.h"
#include "logging.h"
#include "gpio.h"

#include "pico/stdlib.h"

#include <stdio.h>

#define PIN_MASK(...) (0 __VA_ARGS__)
#define PIN_MASK_SET_BIT(bit) | (1ul << bit)

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
      LOG_GPIB("DCL\n");
      break;

    case GPIB_CMD_SPE:
      LOG_GPIB("SPE\n");
      break;

    case GPIB_CMD_SPD:
      LOG_GPIB("SPD\n");
      break;

    case GPIB_CMD_MLA:
      LOG_GPIB("MLA(%d)\n", cmd->addr);
      break;
    
    case GPIB_CMD_UNL:
      LOG_GPIB("UNL\n");
      break;

    case GPIB_CMD_MTA:
      LOG_GPIB("MTA(%d)\n", cmd->addr);
      break;

    case GPIB_CMD_UNT:
      LOG_GPIB("UNT\n");
      break;

    default:
      LOG_GPIB("unknown command 0x%02x\n", cmd->raw);
      break;
  }
}

void gpib_preconfigure_pins(void) {
  #define GPIB_PINS  \
    X(PIN_GPIB_DIO1) \
    X(PIN_GPIB_DIO2) \
    X(PIN_GPIB_DIO3) \
    X(PIN_GPIB_DIO4) \
    X(PIN_GPIB_DIO5) \
    X(PIN_GPIB_DIO6) \
    X(PIN_GPIB_DIO7) \
    X(PIN_GPIB_DIO8) \
    X(PIN_GPIB_REN)  \
    X(PIN_GPIB_IFC)  \
    X(PIN_GPIB_NDAC) \
    X(PIN_GPIB_NRFD) \
    X(PIN_GPIB_DAV)  \
    X(PIN_GPIB_ATN)  \
    X(PIN_GPIB_EOI)  \
    X(PIN_GPIB_SRQ)  \
    X(PIN_GPIB_DC)   \
    X(PIN_GPIB_TE)

  #define X(pin) \
    gpio_set_function(pin, GPIO_FUNC_SIO); \
    gpio_disable_pulls(pin);

    GPIB_PINS
  #undef X

  gpio_set_dir(PIN_GPIB_PE, GPIO_OUT);
  gpio_disable_pulls(PIN_GPIB_PE);
  gpio_put(PIN_GPIB_PE, true);

  gpio_set_dir(PIN_GPIB_TE, GPIO_OUT);
  gpio_disable_pulls(PIN_GPIB_TE);
  gpio_put(PIN_GPIB_TE, true);

  gpio_set_dir(PIN_GPIB_DC, GPIO_OUT);
  gpio_disable_pulls(PIN_GPIB_DC);
  gpio_put(PIN_GPIB_DC, true);
}

void gpib_configure_listener(void) {
  #define OUT_PINS   \
    X(PIN_GPIB_SRQ)  \
    X(PIN_GPIB_NDAC) \
    X(PIN_GPIB_NRFD)

  #define INPUT_PINS  \
    X(PIN_GPIB_DIO1)  \
    X(PIN_GPIB_DIO2)  \
    X(PIN_GPIB_DIO3)  \
    X(PIN_GPIB_DIO4)  \
    X(PIN_GPIB_DIO5)  \
    X(PIN_GPIB_DIO6)  \
    X(PIN_GPIB_DIO7)  \
    X(PIN_GPIB_DIO8)  \
    X(PIN_GPIB_ATN)   \
    X(PIN_GPIB_REN)   \
    X(PIN_GPIB_IFC)   \
    X(PIN_GPIB_EOI)   \
    X(PIN_GPIB_DAV)

  #define X(pin) PIN_MASK_SET_BIT(pin)
    gpio_set_dir_out_masked(PIN_MASK(OUT_PINS));
    gpio_set_mask(PIN_MASK(OUT_PINS));
    gpio_set_dir_in_masked(PIN_MASK(INPUT_PINS));
  #undef X

  #undef INPUT_PINS
  #undef OUT_PINS

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

  while (!gpio_get(PIN_GPIB_DAV));

  gpio_put(PIN_GPIB_NRFD, true);
}

void gpib_end_handshake(void) {
  gpio_put(PIN_GPIB_NRFD, false);
  gpio_put(PIN_GPIB_NDAC, true);

  while (!gpio_get(PIN_GPIB_DAV));

  gpio_put(PIN_GPIB_NDAC, false);
  gpio_put(PIN_GPIB_NRFD, true);
}

gpib_byte_t gpib_start_data_handshake(void) {
  gpio_put(PIN_GPIB_NDAC, false);

  while (gpio_get(PIN_GPIB_DAV));

  uint8_t value = gpib_read_byte();
  bool atn = gpio_get(PIN_GPIB_ATN);
  bool eoi = gpio_get(PIN_GPIB_EOI);

  return (gpib_byte_t){ value, atn, eoi };
}

void gpib_configure_talker(void) {
  #define OUT_PINS    \
    X(PIN_GPIB_DIO1)  \
    X(PIN_GPIB_DIO2)  \
    X(PIN_GPIB_DIO3)  \
    X(PIN_GPIB_DIO4)  \
    X(PIN_GPIB_DIO5)  \
    X(PIN_GPIB_DIO6)  \
    X(PIN_GPIB_DIO7)  \
    X(PIN_GPIB_DIO8)  \
    X(PIN_GPIB_ATN)   \
    X(PIN_GPIB_REN)   \
    X(PIN_GPIB_IFC)   \
    X(PIN_GPIB_EOI)   \
    X(PIN_GPIB_DAV)

  #define INPUT_PINS  \
    X(PIN_GPIB_NDAC)  \
    X(PIN_GPIB_NRFD)  \
    X(PIN_GPIB_SRQ)

  #define X(pin) PIN_MASK_SET_BIT(pin)
    gpio_set_dir_out_masked(PIN_MASK(OUT_PINS));
    gpio_set_mask(PIN_MASK(OUT_PINS));
    gpio_set_dir_in_masked(PIN_MASK(INPUT_PINS));
  #undef X

  #undef INPUT_PINS
  #undef OUT_PINS

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

  while (gpio_get(PIN_GPIB_NDAC) || !gpio_get(PIN_GPIB_NRFD));

  // Now we can signal that data is valid.
  gpio_put(PIN_GPIB_DAV, false);

  // Wait until the laptop signals successful data read.
  while (!gpio_get(PIN_GPIB_NDAC));

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
