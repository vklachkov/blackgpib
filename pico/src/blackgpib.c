#include "blackgpib.h"

#include "common.h"
#include "logging.h"
#include "gpib.h"
#include "gpio.h"
#include "disk_emulator.h"

#include "hardware/gpio.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NO_ACTIVE_DEVICE 0xFF

typedef enum {
  SP_DISABLED,
  SP_UNEXPECTED,
  SP_REQUESTED,
} serial_poll_state_t;

typedef struct blackgpib_s {
  uint8_t active_listener;
  uint8_t active_talker;

  serial_poll_state_t serial_poll_state;
  uint8_t serial_poll_requester;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;

  disk_emulator_t* emulators[MAX_DEVICES];
} blackgpib_t;

blackgpib_t* blackgpib_new(disk_emulator_t* emulators[MAX_DEVICES]) {
  blackgpib_t* blackgpib = malloc(sizeof(blackgpib_t));
  if (blackgpib == NULL) {
    return NULL;
  }

  blackgpib->active_listener = NO_ACTIVE_DEVICE;
  blackgpib->active_talker = NO_ACTIVE_DEVICE;

  blackgpib->serial_poll_state = SP_DISABLED;
  blackgpib->serial_poll_requester = NO_ACTIVE_DEVICE;

  blackgpib->buffer_len = 0;

  memcpy(blackgpib->emulators, emulators, sizeof(disk_emulator_t*) * MAX_DEVICES);

  return blackgpib;
}

static void blackgpib_reset(blackgpib_t* emu) {
  for (int i = 0; i < MAX_DEVICES; i++) {
    disk_emu_reset(emu->emulators[i]);
  }

  emu->active_listener = NO_ACTIVE_DEVICE;
  emu->active_talker = NO_ACTIVE_DEVICE;
  emu->serial_poll_state = SP_DISABLED;
  emu->serial_poll_requester = NO_ACTIVE_DEVICE;
  emu->buffer_len = 0;
}

static void listen_to_buffer(blackgpib_t* emu) {
  emu->buffer_len = 0;

  while (true) {
    gpib_byte_t byte = gpib_start_data_handshake();

    if (!byte.atn) {
      gpib_unexpected_byte();
      blackgpib_reset(emu);
      break;
    }

    if (emu->buffer_len < SECTOR_SIZE) {
      emu->buffer[emu->buffer_len++] = byte.value;
    }

    gpib_end_handshake();

    if (!byte.eoi) {
      break;
    }
  }
}

static void emulator_talk(blackgpib_t* emu) {
  gpib_configure_talker();

  if (emu->serial_poll_state == SP_REQUESTED) {
    uint8_t serial_poll_response =
      emu->active_talker == emu->serial_poll_requester ? 0x4F : 0x0F;

    LOG_TRANSPORT("send serial poll response 0x%02x\n", serial_poll_response);
    gpib_send_serial_poll_response(serial_poll_response);
    LOG_TRANSPORT("send finished\n");
  } else {
    const uint8_t* buffer = NULL;
    size_t size = 0;

    disk_emu_get_talk_bytes(emu->emulators[emu->active_talker], &buffer, &size);
    
    LOG_TRANSPORT("send %zu bytes start\n", size);
    gpib_send_bytes(buffer, size);
    LOG_TRANSPORT("send finished\n");

    disk_emu_reset(emu->emulators[emu->active_talker]);
  }

  gpib_configure_listener();
}

void blackgpib_run(blackgpib_t* emu) {
  gpib_preconfigure_pins();
  gpib_configure_listener();

  while (true) {
    gpib_cmd_t cmd = gpib_start_command_handshake();
    // gpib_cmd_debug(&cmd);

    gpio_put(PIN_LED, 0);

    switch (cmd.type) {
      // Device clear.
      case GPIB_CMD_DCL:
      {
        gpib_end_handshake();
        blackgpib_reset(emu);
        break;
      }

      // Serial poll enable.
      case GPIB_CMD_SPE:
      {
        if (emu->serial_poll_state == SP_DISABLED) {
          emu->serial_poll_state = SP_UNEXPECTED;
        }
        else if (emu->serial_poll_state == SP_REQUESTED) {
          gpib_end_handshake();
        }
        else {
          gpib_unexpected_byte();
        }
        break;
      }

      // Serial poll disable.
      case GPIB_CMD_SPD:
      {
        if (emu->serial_poll_state == SP_DISABLED) {
          gpib_unexpected_byte();
        }
        else if (emu->serial_poll_state == SP_REQUESTED) {
          gpib_end_handshake();
        }
        else {
          gpib_unexpected_byte();
        }

        emu->serial_poll_state = SP_DISABLED;
        emu->serial_poll_requester = NO_ACTIVE_DEVICE;

        break;
      }

      // My listen address.
      case GPIB_CMD_MLA:
      {
        if (emu->emulators[cmd.addr] == NULL) {
          gpib_unexpected_byte();
          break;
        }

        gpib_end_handshake();

        emu->active_listener = cmd.addr;
        listen_to_buffer(emu);

        break;
      }

      // Unlisten.
      case GPIB_CMD_UNL:
      {
        if (emu->active_listener == NO_ACTIVE_DEVICE) {
          gpib_unexpected_byte();
          break;
        }

        gpib_end_handshake();

        bool srq_required = disk_emu_process_buffer(emu->emulators[emu->active_listener],
                                                    emu->buffer, emu->buffer_len);
        if (srq_required) {
          gpio_put(PIN_GPIB_SRQ, false);

          emu->serial_poll_state = SP_REQUESTED;
          emu->serial_poll_requester = emu->active_listener;
        }

        emu->active_listener = NO_ACTIVE_DEVICE;
        emu->buffer_len = 0;

        break;
      }

      // My Talk Address.
      case GPIB_CMD_MTA:
      {
        if (emu->emulators[cmd.addr] == NULL) {
          gpib_unexpected_byte();
          break;
        }

        gpib_end_handshake();

        emu->active_talker = cmd.addr;
        emulator_talk(emu);

        break;
      }

      // Untalk.
      case GPIB_CMD_UNT:
      {
        if (emu->active_talker == NO_ACTIVE_DEVICE) {
          gpib_unexpected_byte();
        } else {
          gpib_end_handshake();
          emu->active_talker = NO_ACTIVE_DEVICE;
        }

        break;
      }

      // Unrecognized byte.
      case GPIB_CMD_UNKNOWN:
      {
        gpib_unexpected_byte();
        break;
      }

      default:
        assert(!"Unhandled GPIB command");
    }
    
    gpio_put(PIN_LED, 1);
  }
}
