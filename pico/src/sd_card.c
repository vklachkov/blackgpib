#include "sd_card.h"

#include "common.h"
#include "gpio.h"

#include "loaders/img.h"
#include "loaders/loader.h"

#include "pico/stdlib.h"
#include "hardware/watchdog.h"
#include "hardware/gpio.h"

#include "pico_fatfs/fatfs/ff.h"
#include "pico_fatfs/tf_card.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>
#include <string.h>
#include <ctype.h>

#define SD_CARD_OP_LOG(...) printf(__VA_ARGS__)

typedef struct {
  bool valid;
  uint8_t address;
  const char* extension;
} parsed_image_fname_t;

const disk_loader_vtable_t* LOADERS[] = {&DISK_IMG_LOADER};
const size_t LOADERS_COUNT = sizeof(LOADERS) / sizeof(disk_loader_vtable_t*);

static FATFS fs = {};

__attribute__((noreturn, noinline))
static void sd_card_fault(void) {
  FATFS fs;

  // unmount previous mount.
  f_unmount("");

  while (true) {
    // try to mount sd card...
    if (f_mount(&fs, "", 1) == FR_OK)
        break;

    // blink on fail.
    gpio_xor_mask(1 << PIN_LED);

    sleep_ms(1000);
  }

  // SD card is reinserted, we give the user an indication.
  gpio_put(PIN_LED, 0);
  for (int i = 0; i < 8; i++) {
    gpio_xor_mask(1 << PIN_LED);
    sleep_ms(100);
  }
  gpio_put(PIN_LED, 0);

  // and go to reboot to reinitialize device.
  while (true) watchdog_reboot(0, 0, 0);
}

void sd_card_init(void) {
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

  pico_fatfs_set_config(&config);

  FRESULT res = f_mount(&fs, "", 1);
  if (res) sd_card_fault();
}

uint8_t sd_card_get_type(void) {
  return fs.fs_type;
}

uint64_t sd_card_get_size(void) {
  return (uint64_t)(fs.n_fatent - 2) * (uint64_t)(fs.csize) * (uint64_t)(SECTOR_SIZE);
}

static parsed_image_fname_t parse_file_name(const char* fname) {
  parsed_image_fname_t ret = {
    .valid = false,
    .address = NO_GPIB_ADDRESS,
    .extension = NULL
  };

  const char* ext = strrchr(fname, '.');
  if (ext == NULL) {
    printf("whatttttttttt? %s\n", fname);
    return ret;
  }

  ret.valid = true;
  ret.extension = ext + 1;  // skip dot


  if (
    (fname[0] != 'H' && fname[0] != 'h') &&
    (fname[0] != 'F' && fname[0] != 'f') &&
    (fname[0] != 'W' && fname[0] != 'w')
  ) {
    printf("skip file name parsing for %s\n", fname);
    return ret;
  }

  ret.address = 0;

  while (*fname != '\0') {
    if (isdigit((int)*fname)) {
      ret.address = (ret.address * 10) + (*fname - '0');
      if (ret.address > 31) {
        printf("baaaaaaad address %d in files %s\n", (int)ret.address, fname);
        ret.valid = false;
        ret.address = NO_GPIB_ADDRESS;
        ret.extension = NULL;
        return ret;
      }
    }
    fname++;
  }

  return ret;
}

sd_card_image_loaders_list_t sd_card_get_image_loaders(void) {
  FRESULT res;

  res = f_mount(&fs, "", 1);
  if (res) sd_card_fault();

  DIR dir;
  FILINFO finfo;

  res = f_opendir(&dir, "/");
  if (res) sd_card_fault();

  sd_card_image_loaders_list_t list = {
    .ptr = malloc(sizeof(sd_card_image_loader_item_t) * MAX_IMAGE_LOADERS),
    .size = 0,
  };

  for (;;) {
    res = f_readdir(&dir, &finfo);
    if (res) {
      sd_card_fault();
    }

    // end of dir.
    if (finfo.fname[0] == '\0') {
      break;
    }

    // skip all subdirectories.
    if (finfo.fattrib & AM_DIR) {
      continue;
    }

    // skip all hidden files.
    if (finfo.fname[0] == '.') {
      printf("skip file %s\n", finfo.fname);
      continue;
    }

    // try to extract address and extension from file name.
    parsed_image_fname_t parsed_fname = parse_file_name(finfo.fname);
    if (!parsed_fname.valid) {
      printf("skip invalid name file %s\n", finfo.fname);
      continue;
    }

    // try to find loader.
    const disk_loader_vtable_t* loader_vtable = NULL;
    for (size_t i = 0; i < LOADERS_COUNT; i++) {
      if (LOADERS[i]->is_supported_ext(parsed_fname.extension)) {
        loader_vtable = LOADERS[i];
        break;
      }
    }

    // unsupported image file.
    if (loader_vtable == NULL) {
      printf("skip unsuported file %s\n", finfo.fname);
      continue;
    }

    sd_card_file_t* file = sd_card_open_file(&finfo);
    assert(file != NULL);

    if (list.size == MAX_IMAGE_LOADERS) {
      SD_CARD_OP_LOG("sd: images limit (%d) has been reached, ignore file '%s'\n", MAX_IMAGE_LOADERS, finfo.fname);
      continue;
    }

    list.ptr[list.size++] = (sd_card_image_loader_item_t) {
      .file_name = file->file_name,
      .gpib_address = parsed_fname.address,
      .loader = (disk_loader_t) {
        .self = loader_vtable->ctor(file),
        .vtable = loader_vtable,
      },
    };
  }

  printf("waaaaaay...\n");

  return list; 
}

sd_card_file_t* sd_card_open_file(FILINFO* finfo) {
  sd_card_file_t* file = malloc(sizeof(sd_card_file_t));
  if (file == NULL) {
    return NULL;
  }

  size_t file_name_size = strlen(finfo->fname) + 1;

  file->file_name = malloc(file_name_size);
  if (file->file_name == NULL) {
    free(file);
    return NULL;
  }

  // if file exists, but we can't open it, assume the sd card is broken.
  FRESULT res = f_open(&file->obj, finfo->fname, FA_READ | FA_WRITE | FA_OPEN_EXISTING);
  if (res != FR_OK) {
    SD_CARD_OP_LOG("sd: failed to open file '%s'\n", finfo->fname);
    sd_card_fault();
  }

  file->size = finfo->fsize;

  memcpy(file->file_name, finfo->fname, file_name_size);

  return file;
}

void sd_card_close_file(sd_card_file_t* file) {
  f_close(&file->obj);
  free(file->file_name);
  free(file);
}

void sd_card_read(sd_card_file_t* file, uint32_t offset, size_t size, uint8_t* out) {
  FRESULT res;
  UINT br;

  res = f_lseek(&file->obj, offset);
  if (res) {
    SD_CARD_OP_LOG("sd: failed to seek file '%s' to offset %" PRIu32 "\n",
                   file->file_name, offset);
    sd_card_fault();
  }

  res = f_read(&file->obj, out, SECTOR_SIZE, &br);
  if (res) {
    SD_CARD_OP_LOG("sd: failed to read %zu bytes from file '%s' at offset %" PRIu32 "\n",
                   size, file->file_name, offset);
    sd_card_fault();
  }

  if (br != size) {
    SD_CARD_OP_LOG("sd: insufficient bytes read. expected %zu, read %zu bytes from file '%s' at offset %" PRIu32 "\n",
                   size, (size_t)br, file->file_name, offset);
    sd_card_fault();
  }
}

void sd_card_write(sd_card_file_t* file, uint32_t offset, const uint8_t* buffer, size_t size) {
  FRESULT res;
  UINT bw;

  res = f_lseek(&file->obj, offset);
  if (res) {
    SD_CARD_OP_LOG("sd: failed to seek file '%s' to offset %" PRIu32 "\n",
                   file->file_name, offset);
    sd_card_fault();
  }

  res = f_write(&file->obj, buffer, SECTOR_SIZE, &bw);
  if (res) {
    SD_CARD_OP_LOG("sd: failed to write %zu bytes from file '%s' at offset %" PRIu32 "\n",
                   size, file->file_name, offset);
    sd_card_fault();
  }

  if (bw != size) {
    SD_CARD_OP_LOG("sd: insufficient bytes write. expected %zu, wrote %zu bytes from file '%s' at offset %" PRIu32 "\n",
                   size, (size_t)bw, file->file_name, offset);
    sd_card_fault();
  }
}
