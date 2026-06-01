#pragma once

#include "pico_fatfs/fatfs/ff.h"

#include <stdbool.h>
#include <stdint.h>

#define SECTOR_SIZE 512

typedef enum {
    LOADER_OK = 0,
    LOADER_IO_ERR,
    LOADER_NOMEM_ERR,
} disk_loader_err_t;

typedef struct {
    uint16_t sector_count;
    uint16_t sectors_per_track;
    uint16_t tracks_per_cylinder;
} disk_geometry_t;

typedef struct {
    bool (*is_supported_ext)(const char* ext);
    disk_loader_err_t (*open)(FATFS* /* fs */, const char* /* path */, void** /* this */);
    disk_loader_err_t (*geometry)(void* /* this */, disk_geometry_t* /* out */);
    disk_loader_err_t (*read)(void* /* this */, uint16_t /* sector */, uint8_t (*)[SECTOR_SIZE] /* out */);
    disk_loader_err_t (*write)(void* /* this */, uint16_t /* sector */, uint8_t (*)[SECTOR_SIZE] /* data */);
} disk_loader_vtable_t;

typedef struct {
    void* self;
    const disk_loader_vtable_t* vtable;
} disk_loader_t;
