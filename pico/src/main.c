#include "blackgpib.h"
#include "gpio.h"
#include "sd_card.h"

#include "pico/stdlib.h"
#include "hardware/gpio.h"

#include <stdio.h>

#define LOG_ERROR(...) printf(__VA_ARGS__)
#define LOG_SD_CARD(...) printf(__VA_ARGS__)

static void configure_led(void) {
  gpio_init(PIN_LED);
  gpio_set_dir(PIN_LED, GPIO_OUT);
  gpio_put(PIN_LED, 1);
}

static void log_sd_card_info(void) {
  switch (sd_card_get_type()) {
    case FS_FAT12:
      LOG_SD_CARD("FS: FAT12\n");
      break;
    case FS_FAT16:
      LOG_SD_CARD("FS: FAT16\n");
      break;
    case FS_FAT32:
      LOG_SD_CARD("FS: FAT32\n");
      break;
    case FS_EXFAT:
      LOG_SD_CARD("FS: ExFAT\n");
      break;
    default:
      LOG_SD_CARD("FS: unknown\n");
      break;
  }

  LOG_SD_CARD("Card size: %0.2f MB\n", sd_card_get_size() / 1024.0 / 1024.0);
}

static void build_emulators(disk_emulator_t* emulators[MAX_DEVICES]) {
  const uint8_t DEAFULT_ADDRESSES[5] = {4, 5, 6, 12, 13};
  const uint8_t FDD_ADDRESSES[3] = {5, 6, 13};

  sd_card_image_loaders_list_t list = sd_card_get_image_loaders();

  printf("1\n");

  for (size_t i = 0; i < list.size; i++) {
    printf("%zu. name = '%s', address = %d", i, list.ptr[i].file_name, (int) list.ptr[i].gpib_address);
  }

  printf("2\n");

  // fill slots according to the file names.
  for (size_t i = 0; i < list.size; i++) {
    sd_card_image_loader_item_t* item = &list.ptr[i];
    if (item->gpib_address != NO_GPIB_ADDRESS) {
      if (emulators[item->gpib_address] == NULL) {
        emulators[item->gpib_address] = disk_emu_new(item->loader);

        LOG_SD_CARD("successfully bind image '%s' to address %d\n",
                    item->file_name, (int)item->gpib_address);
      } else {
        LOG_SD_CARD("failed to bind image '%s' to address %d: address already busy\n",
                    item->file_name, (int)item->gpib_address);
      }
    }
  }

  // fill slots with floppy images.
  for (size_t i = 0; i < list.size; i++) {
    sd_card_image_loader_item_t* item = &list.ptr[i];
    disk_geometry_t geometry = item->loader.vtable->geometry(item->loader.self);

    // skip all files with addresses in file names.
    if (item->gpib_address != NO_GPIB_ADDRESS) {
      continue;
    }

    // all floppy images has 720 sectors (360KiB, DS DD).
    if (geometry.total_sectors != 720) {
      continue;
    }

    uint8_t free_address = NO_GPIB_ADDRESS;

    for (size_t j = 0; j < sizeof(FDD_ADDRESSES); j++) {
      if (emulators[FDD_ADDRESSES[j]] == NULL) {
        free_address = FDD_ADDRESSES[j];
        break;
      }
    }

    if (free_address != NO_GPIB_ADDRESS) {
      emulators[free_address] = disk_emu_new(item->loader);

      LOG_SD_CARD("successfully bind floppy image '%s' to address %d\n",
                  item->file_name, (int)free_address);
    } else {
      LOG_SD_CARD("failed to bind floppy image '%s': no free slots, please specify address in file name\n",
                  item->file_name);
    }
  }

  // fill slots with other images.
  for (size_t i = 0; i < list.size; i++) {
    sd_card_image_loader_item_t* item = &list.ptr[i];
    disk_geometry_t geometry = item->loader.vtable->geometry(item->loader.self);

    // skip all files with addresses in file names.
    if (item->gpib_address != NO_GPIB_ADDRESS) {
      continue;
    }

    // skip all floppy images.
    if (geometry.total_sectors == 720) {
      continue;
    }

    uint8_t free_address = NO_GPIB_ADDRESS;

    for (size_t j = 0; j < sizeof(DEAFULT_ADDRESSES); j++) {
      if (emulators[DEAFULT_ADDRESSES[j]] == NULL) {
        free_address = DEAFULT_ADDRESSES[j];
        break;
      }
    }

    if (free_address != NO_GPIB_ADDRESS) {
      emulators[free_address] = disk_emu_new(item->loader);

      LOG_SD_CARD("successfully bind image '%s' to address %d\n",
                  item->file_name, (int)free_address);
    } else {
      LOG_SD_CARD("failed to bind image '%s': no free slots, please specify address in file name\n",
                  item->file_name);
    }
  }
}

int main() {
  stdio_init_all();

  sleep_ms(1500);

  configure_led();

  sd_card_init();

  log_sd_card_info();

  disk_emulator_t* emulators[MAX_DEVICES] = {0};
  build_emulators(emulators);

  blackgpib_t* blackgpib = blackgpib_new(emulators);

  LOG_SD_CARD("Initialization complete!");

  blackgpib_run(blackgpib);

  return 0;
}
