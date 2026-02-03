#include "blackgpib.h"
#include "disk_protocol.h"

#include "pico/stdlib.h"
#include "pico_fatfs/tf_card.h"
#include "pico_fatfs/fatfs/ff.h"

#include "hardware/gpio.h"

#include <stdio.h>
#include <stdlib.h>

#define SECTOR_SIZE 512

////////////////////////////////////////////////////////////////////////////////

void init_gpio_pins(void) {
  gpio_set_function(PIN_GPIB_TE, GPIO_FUNC_SIO);
  gpio_set_dir(PIN_GPIB_TE, GPIO_OUT);

  gpio_set_function(PIN_GPIB_DC, GPIO_FUNC_SIO);
  gpio_set_dir(PIN_GPIB_DC, GPIO_OUT);
}

__attribute__((always_inline))
static inline void configure_output(uint gpio) {
  gpio_set_function(gpio, GPIO_FUNC_SIO);
  gpio_set_dir(gpio, GPIO_OUT);
  gpio_put(gpio, true);
}

__attribute__((always_inline))
static inline void configure_input(uint gpio) {
  gpio_set_function(gpio, GPIO_FUNC_SIO);
  gpio_set_dir(gpio, GPIO_IN);
  gpio_pull_down(gpio);
}

////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////

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
  bool a = false;

  while (true) {
    if (gpio_get(PIN_GPIB_ATN)) {
      gpio_put(PIN_GPIB_NDAC, true);
      continue;
    }

    if (a == false) {
      printf("ATN now low\n");
      a = true;
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

void gpib_unexpected_command(void) {
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

typedef struct {
  uint8_t value;
  bool atn;
  bool eoi;
} gpib_byte_t;

gpib_byte_t gpib_start_data_handshake(void) {
  gpio_put(PIN_GPIB_NDAC, false);

  while (gpio_get(PIN_GPIB_DAV)) {}

  uint8_t value = gpib_read_byte();
  bool atn = gpio_get(PIN_GPIB_ATN);
  bool eoi = gpio_get(PIN_GPIB_EOI);

  return (gpib_byte_t){ value, atn, eoi };
}

////////////////////////////////////////////////////////////////////////////////

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
    gpio_put(pins[i], byte >> i & 0b1);
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

////////////////////////////////////////////////////////////////////////////////

typedef struct {
  bool has_request;
  disk_req_t current_request;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;
} disk_emulator_t;

void disk_emu_reset(disk_emulator_t* emu) {
  emu->has_request = false;
  emu->buffer_len = 0;
}

void disk_emu_process_new_request(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  emu->has_request = false;

	disk_req_t req;
  
  int ret = disk_req_parse(data, size, &req);
  if (ret) {
		printf("disk_emu: received unusual %zu bytes request, expected %d bytes\n",
			     size, REQUEST_LEN);
		return;
  }

	switch (req.code) {
    case DISK_REQ_INITIALIZE:
      // do nothing, everything is already initialized.
      printf("disk_emu: received Initialize request\n");
      break;

    case DISK_REQ_GET_STATUS:
      // do nothing, this command requires no action.
      printf("disk_emu: received GetStatus(size=%d) request\n", req.data_size);
      break;

    case DISK_REQ_WRITE:
      // after this command, 512 more bytes are expected.
      printf("disk_emu: received Write(sector=%d, mode=%d) request\n", req.sector, req.mode);
      break;

    case DISK_REQ_READ:
      printf("disk_emu: received Read(sector=%d) request\n", req.sector);
      // TODO: Read from sd card.
      emu->buffer_len = 512;
      break;

    case DISK_REQ_FORMAT:
      printf("%s emu: received Format request\n");
      // TODO: Format image on SD card.
      break;

    default:
      printf("%s emu: received unsupported request %d with sector=%d, data_size=%d, mode=%d\n",
             req.code, req.sector, req.data_size, req.mode);
      break;
	}

	emu->current_request = req;
}

void disk_emu_process_buffer(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  if (emu->has_request) {
		if (emu->current_request.code == DISK_REQ_WRITE)
		{
      // TODO: Write data to the disk.
      disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, emu->current_request.sector, 0 };
      disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);

			return;
		}
	}

	disk_emu_process_new_request(emu, data, size);
}

void disk_emu_get_status(disk_emulator_t* emu) {
	disk_status_t status = {0};

	status.sector_size = 512;
	status.logical_sector_size = 504;
	status.sector_count = 720;      // TODO: from disk geometry
	status.drive_status = 1;        // TODO: handle sd card problems
	status.bitmap_fid = 0x120;      // TODO: get from image or guess
	status.superblock_fid = 0x121;  // TODO: get from image or guess
	status.min_dir_pages = 1;
	status.flush = 0;

  for (size_t i = 0; i < sizeof(status.device_name); i++) {
    status.device_name[i] = ' ';
  }

	status.bytes_per_sector = 512;
	status.sectors_per_track = 9; // TODO: get from disk geometry
	status.tracks_per_cylinder = 2; // TODO: get from disk geometry

  emu->buffer_len = disk_status_serialize(&status, emu->buffer, SECTOR_SIZE);
}

void disk_emu_talk(disk_emulator_t* emu) {
  if (emu->has_request) {
		switch (emu->current_request.code) {
      case DISK_REQ_INITIALIZE: {
        disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK };
        disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
        break;
      }

      case DISK_REQ_GET_STATUS: {
        disk_emu_get_status(emu);
        break;
      }

      case DISK_REQ_READ:
      case DISK_REQ_WRITE:
      case DISK_REQ_FORMAT: {
        // data already in buffer.
        break;
      }

      default: {
        disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED };
        disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
        break;
      }
		}
	}
	else
	{
    disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED };
    disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
	}

	if (emu->buffer_len == 0) {
		assert(!"Buffer is not filled with data for laptop");
  }

	gpib_send_bytes((const uint8_t*)&emu->buffer, emu->buffer_len);

	disk_emu_reset(emu);
}

////////////////////////////////////////////////////////////////////////////////

typedef struct {
  uint8_t gpib_address;

  bool listening;
  bool talking;

  bool serial_poll;
  bool srq_raised;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;

  disk_emulator_t disk_emu;
} blackgpib_emulator_t;

static void emulator_reset(blackgpib_emulator_t* emu) {
  disk_emu_reset(&emu->disk_emu);

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
			emulator_reset(emu);
			break;
		}

    if (emu->buffer_len < SECTOR_SIZE) {
      emu->buffer[emu->buffer_len++] = byte.value;
    }

		if (!byte.eoi) {
			break;
		}
	}
}

int emulator_main(blackgpib_emulator_t* emu, FIL* image) {
  init_gpio_pins();
  gpib_configure_listener();

  while (true) {
    gpib_cmd_t cmd = gpib_start_command_handshake();
    printf("Cmd %d\n", cmd);
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
          gpib_unexpected_command();
        }
        break;

      case GPIB_CMD_SPD:
        if (emu->srq_raised) {
          gpib_end_handshake();
        } else {
          gpib_unexpected_command();
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
          gpib_unexpected_command();
        }
        break;

      case GPIB_CMD_UNL:
        if (emu->listening) {
          gpib_end_handshake();

          emu->listening = false;

          disk_emu_process_buffer(
            &emu->disk_emu,
            (const uint8_t*)&emu->buffer,
            emu->buffer_len);
          emu->buffer_len = 0;
        }
        else {
          gpib_unexpected_command();
        }
        break;

      case GPIB_CMD_MTA:
        if (cmd.addr == emu->gpib_address) {
          gpib_end_handshake();

          gpib_configure_talker();

          emu->talking = true;
          if (emu->serial_poll)
            gpib_send_serial_poll_response(emu->srq_raised ? 0x4F : 0x0F);
          else
            disk_emu_talk(&emu->disk_emu);

          gpib_configure_listener();
        }
        else {
          gpib_unexpected_command();
        }
        break;

      case GPIB_CMD_UNT:
        if (emu->talking) {
          gpib_end_handshake();
          emu->talking = false;
        } else {
          gpib_unexpected_command();
        }
        break;

      case GPIB_CMD_UNKNOWN:
			  gpib_unexpected_command();
        break;

      default:
        assert(!"Unhandled GPIB command");
    }
  }

  return 0;
}

////////////////////////////////////////////////////////////////////////////////

int open_demo_image(FIL* output) {
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

  FATFS fs;

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

  ret = f_open(output, "GRID OS.IMG", FA_READ | FA_WRITE | FA_OPEN_EXISTING);
  if (ret != FR_OK) {
    printf("Failed to open GRID OS.IMG\n");
    return 1;
  }

  return 0;
}

// int main() {
// configure_output(PIN_GPIB_ATN);
// while (true) {
//   gpio_put(PIN_GPIB_ATN, true);
//   sleep_ms(2000);
//   gpio_put(PIN_GPIB_ATN, false);
//   sleep_ms(2000);
// }
// }

int main() {
  stdio_init_all();

  sleep_ms(3000);

  FIL demo_image;

  int ret = open_demo_image(&demo_image);
  if (ret) {
    printf("Failed to open demo image\n");
    return 1;
  }

  blackgpib_emulator_t* emulator = calloc(1, sizeof(blackgpib_emulator_t));
  if (emulator == NULL) {
    printf("Failed to alloc memory for emulator\n");
    return 1;
  }

  printf("Starting emulator...\n");

  return emulator_main(emulator, &demo_image);
}
