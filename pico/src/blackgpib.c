#include "blackgpib.h"
#include "common.h"
#include "gpib.h"
#include "gpio.h"
#include "disk_emulator.h"
#include "loaders/img.h"
#include "sd_fault.h"

#include "pico/stdlib.h"
#include "pico_fatfs/tf_card.h"
#include "pico_fatfs/fatfs/ff.h"

#include "hardware/gpio.h"

#include <stdio.h>
#include <stdlib.h>

typedef struct {
  uint8_t gpib_address;

  bool listening;
  bool talking;

  bool serial_poll;
  bool srq_raised;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;

  disk_emulator_t* disk_emu;
} blackgpib_emulator_t;

static void emulator_reset(blackgpib_emulator_t* emu) {
  disk_emu_reset(emu->disk_emu);

  emu->listening = false;
  emu->talking = false;
  emu->serial_poll = false;
  emu->srq_raised = false;
  emu->buffer_len = 0;
}

static void emulator_listen_to_buffer(blackgpib_emulator_t* emu) {
  emu->buffer_len = 0;

  while (true) {
    gpib_byte_t byte = gpib_start_data_handshake();

    if (!byte.atn) {
      gpib_unexpected_byte();
      emulator_reset(emu);
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

int emulator_main(blackgpib_emulator_t* emu) {
  gpib_preconfigure_pins();
  gpib_configure_listener();

  while (true) {
    gpib_cmd_t cmd = gpib_start_command_handshake();
    gpib_cmd_debug(&cmd);

    switch (cmd.type) {
      case GPIB_CMD_DCL:
        gpib_end_handshake();
        emulator_reset(emu);
        break;

      case GPIB_CMD_SPE:
        emu->serial_poll = true;
        if (emu->srq_raised) {
          gpib_end_handshake();
        } else {
          gpib_unexpected_byte();
        }
        break;

      case GPIB_CMD_SPD:
        if (emu->srq_raised) {
          gpib_end_handshake();
        } else {
          gpib_unexpected_byte();
        }
        emu->serial_poll = false;
        emu->srq_raised = false;
        break;

      case GPIB_CMD_MLA:
        if (cmd.addr == emu->gpib_address) {
          gpib_end_handshake();

          emu->listening = true;
          emulator_listen_to_buffer(emu);
        }
        else {
          gpib_unexpected_byte();
        }
        break;

      case GPIB_CMD_UNL:
        if (emu->listening) {
          gpib_end_handshake();

          emu->listening = false;

          bool srq_required = disk_emu_process_buffer(emu->disk_emu,
                                                      emu->buffer, emu->buffer_len);
          if (srq_required) {
            gpio_put(PIN_GPIB_SRQ, false);
            emu->srq_raised = true;
          }

          emu->buffer_len = 0;
        }
        else {
          gpib_unexpected_byte();
        }
        break;

      case GPIB_CMD_MTA:
        if (cmd.addr == emu->gpib_address) {
          gpib_end_handshake();

          gpib_configure_talker();

          emu->talking = true;
          if (emu->serial_poll) {
            gpib_send_serial_poll_response(emu->srq_raised ? 0x4F : 0x0F);
          } else {
            const uint8_t* buffer = NULL;
            size_t size = 0;

            disk_emu_get_talk_bytes(emu->disk_emu, &buffer, &size);
            
            printf("send %zu bytes start\n", size);
            gpib_send_bytes(buffer, size);
            printf("send finished\n");
            disk_emu_reset(emu->disk_emu);
          }

          gpib_configure_listener();
        }
        else {
          gpib_unexpected_byte();
        }
        break;

      case GPIB_CMD_UNT:
        if (emu->talking) {
          gpib_end_handshake();
          emu->talking = false;
        } else {
          gpib_unexpected_byte();
        }
        break;

      case GPIB_CMD_UNKNOWN:
        gpib_unexpected_byte();
        break;

      default:
        assert(!"Unhandled GPIB command");
    }
  }

  return 0;
}

////////////////////////////////////////////////////////////////////////////////

int open_demo_image(disk_loader_t* loader) {
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

  static FATFS fs;

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

  if (DISK_IMG_LOADER.open(&fs, "HD4_DATA.img", &loader->self))
    sd_card_fault();
  
  loader->vtable = &DISK_IMG_LOADER;

  return 0;
}

int main() {
  stdio_init_all();

  gpio_init(PIN_LED);
  gpio_set_dir(PIN_LED, GPIO_OUT);
  gpio_put(PIN_LED, 1);

  disk_loader_t loader;

  int ret = open_demo_image(&loader);
  if (ret) sd_card_fault();

  blackgpib_emulator_t* emulator = calloc(1, sizeof(blackgpib_emulator_t));
  if (emulator == NULL) {
    // printf("Failed to alloc memory for emulator\n");
    return 1;
  }

  emulator->gpib_address = 0x04;
  emulator->disk_emu = disk_emu_new(loader);

  printf("Starting emulator...\n");

  return emulator_main(emulator);
}
