#include "disk_emulator.h"
#include "disk_protocol.h"
#include "loaders/loader.h"
#include "common.h"
#include "logging.h"
#include "gpio.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

#define DRIVE_STATUS_READY 1

typedef struct disk_emulator {
  disk_loader_t loader;

  bool has_request;
  disk_req_t current_request;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;
} disk_emulator_t;

disk_emulator_t* disk_emu_new(disk_loader_t loader) {
  disk_emulator_t* emu = calloc(1, sizeof(disk_emulator_t));
  hard_assert(emu != NULL);

  emu->loader = loader;
  return emu;
} 

void disk_emu_reset(disk_emulator_t* emu) {
  emu->has_request = false;
  emu->buffer_len = 0;
}

static void process_write_request(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  if (size != SECTOR_SIZE) {
    LOG_DISK_EMU("received malformed write request. Expected %zu bytes, got %zu", SECTOR_SIZE, size);

    disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED, 0, 0, 0 };
    emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);

    return;
  }

  if (emu->current_request.mode == 1) {
    disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, 0xFFFF, 0 };
    emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);
    return;
  }

  uint32_t sector = emu->current_request.sector;

  emu->loader.vtable->write(emu->loader.self, sector, (void*)data);

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, (uint16_t)sector, 0 };
  emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);

  return;
}

static void process_init_request(disk_emulator_t* emu) {
  // do nothing, everything is already initialized.

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, 0, 0 };
  emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);
}

static uint16_t get_superblock_fid(disk_geometry_t* geometry) {
  if (geometry->total_sectors >= 10240) {  // 5 MB
    return 0x2420;
  } else {
    return 0x121;
  }
}

static uint16_t get_bitmap_fid(disk_geometry_t* geometry) {
  // algorithm is taken from CCOS-disk-utils:
  // https://github.com/BOOtak/CCOS-disk-utils/blob/9d353997dbc0b48993a4ec3af6464a020b43fec2/ccos_format.c#L60

  const uint16_t superblock = get_superblock_fid(geometry);

  const uint16_t required_bytes = geometry->total_sectors / 8;

  // value is taken from CCOS-disk-utils:
  // https://github.com/BOOtak/CCOS-disk-utils/blob/9d353997dbc0b48993a4ec3af6464a020b43fec2/ccos_structure.h#L36
  const uint16_t bytes_per_sector = 512 - 4 - 2 - 2 - 4;

  // round up required sectors.
  const uint16_t count = required_bytes / bytes_per_sector + 1;

  return superblock - count;
}

static uint16_t get_min_dir_pages(disk_geometry_t* geometry) {
  if (geometry->total_sectors >= 10240) {  // 5 MB
    return 10;
  } else {
    return 1;
  }
}

static void process_get_status_request(disk_emulator_t* emu) {
  disk_geometry_t geometry = emu->loader.vtable->geometry(emu->loader.self);

  // TODO: should we support flush?
  disk_status_t status = {
    .sector_size = SECTOR_SIZE,
    .logical_sector_size = LOGICAL_SECTOR_SIZE,
    .sector_count = geometry.total_sectors,
    .drive_status = DRIVE_STATUS_READY,
    .bitmap_fid = get_bitmap_fid(&geometry),
    .superblock_fid = get_superblock_fid(&geometry),
    .min_dir_pages = get_min_dir_pages(&geometry),
    .flush = 0,
    .device_name = {},
    .bytes_per_sector = SECTOR_SIZE,
    .sectors_per_track = geometry.sectors,
    .tracks_per_cylinder = geometry.heads,
  };

  for (size_t i = 0; i < sizeof(status.device_name); i++) {
    status.device_name[i] = ' ';  // TODO: set some name
  }

  emu->buffer_len = disk_status_serialize(&status, emu->buffer, SECTOR_SIZE);
}

static void process_read_request(disk_emulator_t* emu, const disk_req_t* req) {
  uint32_t sector = req->sector;

  emu->loader.vtable->read(emu->loader.self, sector, &emu->buffer);

  emu->buffer_len = SECTOR_SIZE;

  return;
}

static void process_format_request(disk_emulator_t* emu) {
  emu->loader.vtable->format(emu->loader.self);

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, 0, 0 };
  emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);

  gpio_put(PIN_GPIB_SRQ, false);

  return;
}

static void unsupported_request(disk_emulator_t* emu) {
  disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED, 0, 0, 0 };
  emu->buffer_len = disk_resp_serialize(&resp, emu->buffer, SECTOR_SIZE);
}

static bool process_new_request(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  emu->has_request = false;

  disk_req_t req;
  bool srq_required = false;
  
  int ret = disk_req_parse(data, size, &req);
  if (ret) {
    LOG_DISK_EMU("received unusual %zu bytes request. Expected %d bytes, got %zu\n",
            size, REQUEST_LEN, size);
    return srq_required;
  }

  switch (req.code) {
    case DISK_REQ_INITIALIZE:
      LOG_DISK_EMU("received Initialize request\n");
      process_init_request(emu);
      break;

    case DISK_REQ_GET_STATUS:
      LOG_DISK_EMU("received GetStatus(size=%" PRIu16 ") request\n", req.data_size);
      process_get_status_request(emu);
      break;

    case DISK_REQ_WRITE:
      // after this command, 512 more bytes are expected.
      LOG_DISK_EMU("received Write(sector=%" PRIu32 ", mode=%" PRIu8 ") request\n", req.sector, req.mode);
      break;

    case DISK_REQ_READ:
      LOG_DISK_EMU("received Read(sector=%" PRIu32 ") request\n", req.sector);
      process_read_request(emu, &req);
      srq_required = true;
      break;

    case DISK_REQ_FORMAT:
      LOG_DISK_EMU("received Format request\n");
      process_format_request(emu);
      srq_required = true;
      break;

    default:
      LOG_DISK_EMU("received unsupported request %" PRIu8 " with sector=%" PRIu32 ", data_size=%" PRIu16 ", mode=%" PRIu8 "\n",
              req.code, req.sector, req.data_size, req.mode);
      unsupported_request(emu);
      break;
  }

  emu->has_request = true;
  emu->current_request = req;

  return srq_required;
}

bool disk_emu_process_buffer(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  if (emu->has_request) {
    if (emu->current_request.code == DISK_REQ_WRITE)
    {
      process_write_request(emu, data, size);
      return true;
    }
  }

  return process_new_request(emu, data, size);
}

void disk_emu_get_talk_bytes(disk_emulator_t* emu, const uint8_t** bufptr, size_t* bufsize) {
  *bufptr = emu->buffer;
  *bufsize = emu->buffer_len;
}
