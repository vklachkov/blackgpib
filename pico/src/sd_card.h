#pragma once

#include "loaders/loader.h"
#include "sd_card_file.h"

#include "pico_fatfs/fatfs/ff.h"

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

// I can't imagine a scenario where someone would store so many images
// on a sd card, but let's assume it happens.
#define MAX_IMAGE_LOADERS 48

#define NO_GPIB_ADDRESS 0xff

typedef struct {
    char* file_name;
    uint8_t gpib_address;
    disk_loader_t loader;
} sd_card_image_loader_item_t;

typedef struct {
    sd_card_image_loader_item_t* ptr;
    size_t size;
} sd_card_image_loaders_list_t;

void sd_card_init(void);

uint8_t sd_card_get_type(void);

uint64_t sd_card_get_size(void);

void sd_card_log_init(void);

void sd_card_log(const char *format, ...);

sd_card_image_loaders_list_t sd_card_get_image_loaders(void);

sd_card_file_t* sd_card_open_file(FILINFO* finfo);

void sd_card_close_file(sd_card_file_t* file);

void sd_card_read(sd_card_file_t* file, uint32_t offset, size_t size, uint8_t* out);

void sd_card_write(sd_card_file_t* file, uint32_t offset, const uint8_t* buffer, size_t size);
