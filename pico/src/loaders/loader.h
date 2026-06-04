#pragma once

#include "../common.h"
#include "../sd_card_file.h"

#include <stdbool.h>
#include <stdint.h>

typedef struct {
  uint16_t total_sectors;
  uint16_t cylinders;
  uint16_t heads;
  uint16_t sectors;
} disk_geometry_t;

typedef struct {
  bool (*is_supported_ext)(const char* ext);
  void* (*ctor)(sd_card_file_t* /* file */);
  void  (*dtor)(void* /* this */);
  disk_geometry_t (*geometry)(void* /* this */);
  void  (*read)(void* /* this */, uint16_t /* sector */, uint8_t (*)[SECTOR_SIZE] /* out */);
  void  (*write)(void* /* this */, uint16_t /* sector */, uint8_t (*)[SECTOR_SIZE] /* data */);
  void  (*format)(void* /* this */);
} disk_loader_vtable_t;

typedef struct {
  void* self;
  const disk_loader_vtable_t* vtable;
} disk_loader_t;
