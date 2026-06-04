#pragma once

#include "disk_emulator.h"

#define MAX_DEVICES 31

struct blackgpib_s;
typedef struct blackgpib_s blackgpib_t;

blackgpib_t* blackgpib_new(disk_emulator_t* emulators[MAX_DEVICES]);

void blackgpib_run(blackgpib_t* emu);
