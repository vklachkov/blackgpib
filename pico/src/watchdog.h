#pragma once

#include <stdbool.h>

__attribute__((noreturn))
void wd_reboot(void);
__attribute__((noreturn))
void wd_reboot_to_adapter(void);
__attribute__((noreturn))
void wd_reboot_to_emulator(void);

unsigned int wd_get_reboot_count(void);

void wd_reset_reboot_count(void);
bool wd_take_adapter_mode(void);
