#include "watchdog.h"

#include "hardware/watchdog.h"

#define SCRATCH_REBOOT_COUNT  0
#define SCRATCH_MODE          1
#define ADAPTER_MAGIC         0xFFAABBCC

__attribute__((noreturn))
void wd_reboot() {
    watchdog_hw->scratch[SCRATCH_REBOOT_COUNT]++;

    watchdog_reboot(0, 0, 0);
    while (true) tight_loop_contents();
}

void wd_reboot_to_adapter() {
    watchdog_hw->scratch[SCRATCH_MODE] = ADAPTER_MAGIC;
    wd_reboot();
}

void wd_reboot_to_emulator() {
    watchdog_hw->scratch[SCRATCH_MODE] = 0;
    wd_reboot();
}

unsigned int wd_get_reboot_count() {
    return (unsigned int)watchdog_hw->scratch[SCRATCH_REBOOT_COUNT];
}

void wd_reset_reboot_count() {
    watchdog_hw->scratch[SCRATCH_REBOOT_COUNT] = 0;
}

bool wd_take_adapter_mode() {
    bool adapter = watchdog_hw->scratch[SCRATCH_MODE] == ADAPTER_MAGIC;
    watchdog_hw->scratch[SCRATCH_MODE] = 0;
    return adapter;
}
