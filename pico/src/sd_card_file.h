#pragma once

#include "pico_fatfs/fatfs/ff.h"

typedef struct {
    FIL obj;
    uint32_t size;
    char* file_name;
} sd_card_file_t;
