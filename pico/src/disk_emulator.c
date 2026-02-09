#include "disk_emulator.h"
#include "disk_protocol.h"
#include "common.h"
#include "gpio.h"

#include "pico_fatfs/fatfs/ff.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct disk_emulator {
  FIL* file;

  bool has_request;
  disk_req_t current_request;

  uint8_t buffer[SECTOR_SIZE];
  size_t buffer_len;
} disk_emulator_t;

disk_emulator_t* disk_emu_new(FIL* file) {
  disk_emulator_t* emu = calloc(1, sizeof(disk_emulator_t));
  emu->file = file;
  return emu;
} 

void disk_emu_reset(disk_emulator_t* emu) {
  emu->has_request = false;
  emu->buffer_len = 0;
}

void disk_emu_process_write_request(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  if (size != SECTOR_SIZE) {
    printf("gpib_emu: received malformed write request. Expected %d bytes, got %d", SECTOR_SIZE, size);

    disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED };
    emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);

    return;
  }

  if (emu->current_request.mode == 1) {
    disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, 0xFFFF, 0 };
    emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
    return;
  }

  uint32_t sector = emu->current_request.sector;

  UINT bw = 0;

  // TODO: Handle errors.
  f_lseek(emu->file, sector * SECTOR_SIZE);
  f_write(emu->file, data, size, &bw);

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK, 0, (uint16_t)sector, 0 };
  emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);

  return;
}

void disk_emu_process_init_request(disk_emulator_t* emu, const disk_req_t* req) {
  // do nothing, everything is already initialized.

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK };
  emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
}

void disk_emu_process_get_status_request(disk_emulator_t* emu, const disk_req_t* req) {
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
    status.device_name[i] = ' ';  // TODO: Set some name
  }

  status.bytes_per_sector = 512;
  status.sectors_per_track = 9; // TODO: get from disk geometry
  status.tracks_per_cylinder = 2; // TODO: get from disk geometry

  emu->buffer_len = disk_status_serialize(&status, emu->buffer, SECTOR_SIZE);
}

void disk_emu_process_read_request(disk_emulator_t* emu, const disk_req_t* req) {
  uint32_t sector = req->sector;

  // TODO: Handle errors.
  FRESULT res = f_lseek(emu->file, sector * SECTOR_SIZE);
  printf("read, f_lseek: %d\n", res);
  res = f_read(emu->file, emu->buffer, SECTOR_SIZE, &emu->buffer_len);
  printf("read, f_read: %d\n", res);
  emu->buffer_len = 512;

  return;
}

void disk_emu_process_format_request(disk_emulator_t* emu, const disk_req_t* req) {
  // TODO: Format image.

  disk_resp_t resp = (disk_resp_t) { DISK_RESP_OK };
  emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);

  gpio_put(PIN_GPIB_SRQ, false);

  return;
}

void disk_emu_unsupported_request(disk_emulator_t* emu) {
  disk_resp_t resp = (disk_resp_t) { DISK_RESP_UNSUPPORTED };
  emu->buffer_len = disk_resp_serialize(&resp, (uint8_t*)&emu->buffer, SECTOR_SIZE);
}

bool disk_emu_process_new_request(disk_emulator_t* emu, const uint8_t* data, size_t size) {
  emu->has_request = false;

  disk_req_t req;
  bool srq_required = false;
  
  int ret = disk_req_parse(data, size, &req);
  if (ret) {
    printf("disk_emu: received unusual %zu bytes request. Expected %d bytes, got %d\n",
           size, REQUEST_LEN, size);
    return srq_required;
  }

  switch (req.code) {
    case DISK_REQ_INITIALIZE:
      printf("disk_emu: received Initialize request\n");
      disk_emu_process_init_request(emu, &req);
      break;

    case DISK_REQ_GET_STATUS:
      printf("disk_emu: received GetStatus(size=%d) request\n", req.data_size);
      disk_emu_process_get_status_request(emu, &req);
      break;

    case DISK_REQ_WRITE:
      // after this command, 512 more bytes are expected.
      printf("disk_emu: received Write(sector=%ld, mode=%d) request\n", req.sector, req.mode);
      break;

    case DISK_REQ_READ:
      printf("disk_emu: received Read(sector=%ld) request\n", req.sector);
      disk_emu_process_read_request(emu, &req);
      srq_required = true;
      break;

    case DISK_REQ_FORMAT:
      printf("disk_emu: received Format request\n");
      disk_emu_process_format_request(emu, &req);
      srq_required = true;
      break;

    default:
      printf("disk_emu: received unsupported request %d with sector=%ld, data_size=%d, mode=%d\n",
             req.code, req.sector, req.data_size, req.mode);
      disk_emu_unsupported_request(emu);
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
      disk_emu_process_write_request(emu, data, size);
      return true;
    }
  }

  return disk_emu_process_new_request(emu, data, size);
}

void disk_emu_get_talk_bytes(disk_emulator_t* emu, const uint8_t** bufptr, size_t* bufsize) {
  *bufptr = emu->buffer;
  *bufsize = emu->buffer_len;
}
