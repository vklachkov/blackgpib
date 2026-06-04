#include "img.h"

#include "../sd_card.h"

#include <assert.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  sd_card_file_t* file;
  disk_geometry_t geometry;
} img_loader_t;

static bool is_supported_ext(const char* ext) {
  return strncasecmp(ext, "img", 3) == 0;
}

static void* ctor(sd_card_file_t* file) {
  img_loader_t* loader = malloc(sizeof(img_loader_t));
  assert(loader != NULL);

  loader->file = file;
  loader->geometry = (disk_geometry_t) {0};

  return loader;
}

static void dtor(void* this) {
  img_loader_t *thiz = this;

  sd_card_close_file(thiz->file);
  free(thiz);
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

static disk_geometry_t geometry(void* this) {
  img_loader_t *thiz = this;

  if (thiz->geometry.total_sectors == 0) {
    thiz->geometry = guess_geometry(thiz->file->size);
  }

  return thiz->geometry;
}

static void read(void* this, uint16_t sector, uint8_t (*out)[SECTOR_SIZE]) {
  img_loader_t *thiz = this;

  sd_card_read(thiz->file, sector * SECTOR_SIZE, SECTOR_SIZE, (uint8_t*)out);
}

static void write(void* this, uint16_t sector, uint8_t (*data)[SECTOR_SIZE]) {
  img_loader_t *thiz = this;

  sd_card_write(thiz->file, sector * SECTOR_SIZE, (const uint8_t*)data, SECTOR_SIZE);
}

static void format(void* this) {
  img_loader_t *thiz = this;

  disk_geometry_t geom = geometry(this);

  uint8_t buffer[SECTOR_SIZE];
  memset(buffer, 0xE5, SECTOR_SIZE);
  memset(buffer, 0xFF, 8);

  for (uint16_t i = 0; i < geom.total_sectors; i++) {
    sd_card_write(thiz->file, i * SECTOR_SIZE, buffer, SECTOR_SIZE);
  }
}

const disk_loader_vtable_t DISK_IMG_LOADER = {
  .is_supported_ext = is_supported_ext,
  .ctor = ctor,
  .dtor = dtor,
  .geometry = geometry,
  .read = read,
  .write = write,
  .format = format,
};
