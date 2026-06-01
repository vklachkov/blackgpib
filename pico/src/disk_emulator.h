#pragma once

#include "loaders/loader.h"

#include "pico_fatfs/fatfs/ff.h"

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

typedef struct disk_emulator disk_emulator_t;

disk_emulator_t* disk_emu_new(disk_loader_t loader);

void disk_emu_reset(disk_emulator_t* emu);

bool disk_emu_process_buffer(disk_emulator_t* emu, const uint8_t* data, size_t size);

void disk_emu_get_talk_bytes(disk_emulator_t* emu, const uint8_t** bufptr, size_t* bufsize);
