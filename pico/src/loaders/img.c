#include "img.h"

#include <stdlib.h>

typedef struct {
    FIL file;
    disk_geometry_t geometry;
} img_loader_t;

static bool is_supported_ext(const char* ext) {
    return false;
}

static disk_loader_err_t open(FATFS* fs, const char* path, void** this) {
    FRESULT res;

    img_loader_t* loader = malloc(sizeof(img_loader_t));
    if (loader == NULL)
        return LOADER_NOMEM_ERR;

    res = f_open(&loader->file, path, FA_READ | FA_WRITE | FA_OPEN_EXISTING);
    if (res != FR_OK)
        return LOADER_IO_ERR;

    loader->geometry = (disk_geometry_t) {0};

    *this = loader;

    return LOADER_OK;
}

static disk_geometry_t guess_geometry(uint32_t length) {
  disk_geometry_t geometry = {
    .total_sectors = length / SECTOR_SIZE,
    .cylinders = 0,
    .heads = 0,
    .sectors = 0,
  };

  // algorithm is taken from MAME, from src/lib/util/harddisk.cpp.
  for (uint32_t totalsectors = geometry.total_sectors; ; totalsectors++) {
    for (uint32_t cursectors = 63; cursectors > 1; cursectors--) {
      if (totalsectors % cursectors == 0) {
        uint32_t totalheads = totalsectors / cursectors;
        for (uint32_t curheads = 16; curheads > 1; curheads--) {
          if (totalheads % curheads == 0) {
            geometry.cylinders = totalheads / curheads;
            geometry.heads = curheads;
            geometry.sectors = cursectors;
            return geometry;
          }
        }
      }
    }
  }

  return geometry;
}

static disk_loader_err_t geometry(void *this, disk_geometry_t *out) {
    img_loader_t *thiz = this;

    if (thiz->geometry.total_sectors == 0) {
      thiz->geometry = guess_geometry(f_size(&thiz->file));
    }

    *out = thiz->geometry;

    return LOADER_OK;
}

static disk_loader_err_t read(void *this, uint16_t sector, uint8_t (*out)[SECTOR_SIZE]) {
    img_loader_t *thiz = this;

    FRESULT res;
    UINT br;

    res = f_lseek(&thiz->file, sector * SECTOR_SIZE);
    if (res)
        return LOADER_IO_ERR;

    res = f_read(&thiz->file, out, SECTOR_SIZE, &br);
    if (res)
        return LOADER_IO_ERR;

    if (br != SECTOR_SIZE)
        return LOADER_IO_ERR;

    return LOADER_OK;
}

static disk_loader_err_t write(void *this, uint16_t sector, uint8_t (*data)[SECTOR_SIZE]) {
    img_loader_t *thiz = this;

    FRESULT res;
    UINT bw;

    res = f_lseek(&thiz->file, sector * SECTOR_SIZE);
    if (res)
        return LOADER_IO_ERR;

    res = f_write(&thiz->file, data, SECTOR_SIZE, &bw);
    if (res)
        return LOADER_IO_ERR;

    if (bw != SECTOR_SIZE)
        return LOADER_IO_ERR;

    return LOADER_OK;
}

const disk_loader_vtable_t DISK_IMG_LOADER = {
    .is_supported_ext = is_supported_ext,
    .open = open,
    .geometry = geometry,
    .read = read,
    .write = write,
};
