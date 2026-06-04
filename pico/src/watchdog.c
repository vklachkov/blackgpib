#include "watchdog.h"

#include "hardware/watchdog.h"

#define REBOOT_MAGIC  0xAABEBA55u

#define SCRATCH_REBOOT_COUNT  0

__attribute__((noreturn))
void wd_reboot() {
    watchdog_hw->scratch[SCRATCH_REBOOT_COUNT]++;

    watchdog_reboot(0, 0, 0);
    while (true) tight_loop_contents();
}

unsigned int wd_get_reboot_count() {
    return (unsigned int)watchdog_hw->scratch[SCRATCH_REBOOT_COUNT];
}

void wd_reset_reboot_count() {
    watchdog_hw->scratch[SCRATCH_REBOOT_COUNT] = 0;
}
