#include "disk_protocol.h"

int disk_req_parse(const uint8_t* data, const size_t size, disk_req_t* output) {
	if (size != REQUEST_LEN) {
		return 1;
	}

	output->code = data[0];
	output->unused = data[1];
	output->connection = data[2];
	output->sector = (uint32_t)data[3] |
                  ((uint32_t)data[4] << 8) |
                  ((uint32_t)data[5] << 16) |
                  ((uint32_t)data[6] << 24);
	output->data_size = (uint16_t)data[7] | ((uint16_t)data[8] << 8);
	output->mode = data[9];

  return 0;
}

int disk_resp_serialize(const disk_resp_t* resp, uint8_t* output, size_t size) {
    if (size < RESPONSE_LEN) {
        return 0;
    }

	output[0] = resp->status & 0xFF;
	output[1] = (resp->status >> 8) & 0xFF;
	output[2] = resp->unknown;
	output[3] = resp->sector & 0xFF;
	output[4] = (resp->sector >> 8) & 0xFF;
	output[5] = resp->unused & 0xFF;
	output[6] = (resp->unused >> 8) & 0xFF;

  return RESPONSE_LEN;
}

int disk_status_serialize(const disk_status_t* status, uint8_t* output, size_t size) {
  if (size < STATUS_LEN) {
    return 0;
  }

  output[0] = status->sector_size & 0xFF;
	output[1] = (status->sector_size >> 8) & 0xFF;
	output[2] = status->logical_sector_size & 0xFF;
	output[3] = (status->logical_sector_size >> 8) & 0xFF;
	output[4] = status->sector_count & 0xFF;
	output[5] = (status->sector_count >> 8) & 0xFF;
	output[6] = status->drive_status;
	output[7] = status->bitmap_fid & 0xFF;
	output[8] = (status->bitmap_fid >> 8) & 0xFF;
	output[9] = status->superblock_fid & 0xFF;
	output[10] = (status->superblock_fid >> 8) & 0xFF;
	output[11] = status->min_dir_pages & 0xFF;
	output[12] = (status->min_dir_pages >> 8) & 0xFF;
	output[13] = status->flush;

	for (size_t i = 0; i < 32; ++i)
		output[14 + i] = status->device_name[i];

	output[46] = status->bytes_per_sector & 0xFF;
	output[47] = (status->bytes_per_sector >> 8) & 0xFF;
	output[48] = status->sectors_per_track & 0xFF;
	output[49] = (status->sectors_per_track >> 8) & 0xFF;
	output[50] = status->tracks_per_cylinder & 0xFF;
	output[51] = (status->tracks_per_cylinder >> 8) & 0xFF;

  return STATUS_LEN;
}
