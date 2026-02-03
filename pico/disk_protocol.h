#pragma once

#include <stddef.h>
#include <stdint.h>

#define REQUEST_LEN   10
#define RESPONSE_LEN  7
#define STATUS_LEN    52

typedef enum {
	DISK_REQ_INITIALIZE   = 0,
	DISK_REQ_GET_STATUS   = 1,
	DISK_REQ_READ         = 4,
	DISK_REQ_WRITE        = 5,
	DISK_REQ_FORMAT       = 17
} disk_req_code;

typedef struct {
	// Operation code. Determines what magic the emulator will do next.
	uint8_t code;

	// The exact purpose is unknown.
	uint8_t unused;

	// The exact purpose is unknown.
	uint8_t connection;

	// Sector number. Only used for Read, Write, and TrackFormat operations.
	uint32_t sector;

	// Request data size.
	// For Format, it must be 1.
	// For GetStatus, it can be 52, 54, or 56.
	// For Read and Write, it should always be 512.
	uint16_t data_size;

	// Determines what action the command will do.
	// For example, Write with mode=1 is a verification of the received data.
	// Or, for SelfTest, mode=7 turns the drive on, mode=8 turns the drive power off.
	uint8_t mode;
} disk_req_t;

int disk_req_parse(const uint8_t* data, const size_t size, disk_req_t* output);

typedef enum {
	DISK_RESP_OK            = 0x00,
	DISK_RESP_UNSUPPORTED   = 0x23,
	DISK_RESP_NOT_READY     = 0x6b,
	DISK_RESP_OUT_OF_BOUNDS = 0x66,
	DISK_RESP_BAD_SECTOR    = 0x67,
	DISK_RESP_NOT_FORMATTED = 0x68
} disk_resp_status_t;

typedef struct {
	// Status code of the request.
	uint16_t status;

	// Exact purpose is unknown, maybe connection or drive init flag, on real drive always 0.
	uint8_t unknown;

	// Sector number from the request, if needed.
	uint16_t sector;

	// Unused, always 0.
	uint16_t unused;
} disk_resp_t;

int disk_resp_serialize(const disk_resp_t* resp, uint8_t* output, size_t size);

typedef struct {
	// Actual sector size. Always 512 bytes.
	uint16_t sector_size;

	// Number of bytes in a sector that can be used for data. Always 504 bytes.
	uint16_t logical_sector_size;

	// Number of sectors.
	// Must match real value, because CCOS checks
	// disk boundaries when working with it.
	uint16_t sector_count;

	// Status of the drive. 0 is not ready, 1 is ready, 3 is error.
	uint8_t drive_status;

	// Bitmap block number. Always 0x120 (one less than the superblock).
	// Used only in CCOS.
	uint16_t bitmap_fid;

	// Superblock number. Always 0x121.
	// Used only in CCOS.
	uint16_t superblock_fid;

	// Unknown purpose. On 2101 always 1.
	uint16_t min_dir_pages;

	// Unknown purpose. On 2101 always 0.
	uint8_t flush;

	// Device name. Not shown in the CCOS interface, can be anything.
	uint8_t device_name[32];

	// Same as sector_size.
	uint16_t bytes_per_sector;

	// Unknown purpose. Can be 0.
	uint16_t sectors_per_track;

	// Unknown purpose. Can be 0.
	uint16_t tracks_per_cylinder;
} disk_status_t;

int disk_status_serialize(const disk_status_t* status, uint8_t* output, size_t size);
