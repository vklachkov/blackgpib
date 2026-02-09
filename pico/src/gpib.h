#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

typedef enum {
  GPIB_CMD_DCL,
  GPIB_CMD_SPE,
  GPIB_CMD_SPD,
  GPIB_CMD_MLA,
  GPIB_CMD_UNL,
  GPIB_CMD_MTA,
  GPIB_CMD_UNT,
  GPIB_CMD_UNKNOWN = 0xFF,
} gpib_cmd_type_t;

typedef struct {
  uint8_t raw;
  gpib_cmd_type_t type;
  uint8_t addr;
} gpib_cmd_t;

typedef struct {
  uint8_t value;
  bool atn;
  bool eoi;
} gpib_byte_t;

gpib_cmd_t gpib_parse_cmd(uint8_t value);

void gpib_cmd_debug(const gpib_cmd_t* cmd);

void gpib_configure_control_pins(void);

void gpib_configure_listener(void);

uint8_t gpib_read_byte(void);

gpib_cmd_t gpib_start_command_handshake(void);

gpib_byte_t gpib_start_data_handshake(void);

void gpib_unexpected_byte(void);

void gpib_end_handshake(void);

void gpib_configure_talker(void);

void gpib_write_byte(uint8_t byte);

void gpib_send_byte(uint8_t byte, bool eoi);

void gpib_send_serial_poll_response(uint8_t byte);

void gpib_send_bytes(const uint8_t *data, size_t size);
