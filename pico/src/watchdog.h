#pragma once

__attribute__((noreturn))
void wd_reboot(void);

unsigned int wd_get_reboot_count(void);

void wd_reset_reboot_count(void);
