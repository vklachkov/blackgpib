#pragma once

// gpib commands logs.
// NB: only for debugging purposes, as it slows down the emulator and triggers GRiD Compass bugs.
#define LOG_GPIB(...) // printf("gpib: " __VA_ARGS__)

// gpib transport logs.
// NB: only for debugging purposes, as it slows down the emulator and triggers GRiD Compass bugs.
#define LOG_TRANSPORT(...) // printf("gpib: " __VA_ARGS__)

// disk emulator request processing logs.
// NB: only for debugging purposes, as it slows down the emulator and triggers GRiD Compass bugs.
#define LOG_DISK_EMU(...) // printf("disk_emu: " __VA_ARGS__)

// just fatal errors.
#define LOG_FATAL(...) printf("fatal: " __VA_ARGS__)

// logs that will be written to a file on the SD card.
#define LOG_SD_CARD(...) sd_card_log(__VA_ARGS__)

// sd card file scan logs.
#define LOG_SD_CARD_LS(...) printf("sd: " __VA_ARGS__)
