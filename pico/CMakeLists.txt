cmake_minimum_required(VERSION 3.13)

set(CMAKE_C_STANDARD 11)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Initialise pico_sdk from installed location
# (note this can come from environment, CMake cache etc)

# == DO NOT EDIT THE FOLLOWING LINES for the Raspberry Pi Pico VS Code Extension to work ==
if(WIN32)
    set(USERHOME $ENV{USERPROFILE})
else()
    set(USERHOME $ENV{HOME})
endif()
set(sdkVersion 2.2.0)
set(toolchainVersion 14_2_Rel1)
set(picotoolVersion 2.2.0-a4)
set(picoVscode ${USERHOME}/.pico-sdk/cmake/pico-vscode.cmake)
if (EXISTS ${picoVscode})
    include(${picoVscode})
endif()
# ====================================================================================

set(PICO_BOARD pico CACHE STRING "Board type")

include(pico_sdk_import.cmake)

project(blackgpib C CXX ASM)

pico_sdk_init()

add_executable(blackgpib
    src/main.c
    src/blackgpib.c
    src/disk_emulator.c
    src/disk_protocol.c
    src/gpib.c
    src/sd_card.c
    src/loaders/img.c
    src/watchdog.c)

target_compile_options(blackgpib PRIVATE -Wall -Wextra)

add_subdirectory(pico_fatfs)

pico_set_program_name(blackgpib "blackgpib")
pico_set_program_version(blackgpib "4.0")

pico_enable_stdio_uart(blackgpib 0)
pico_enable_stdio_usb(blackgpib 1)

target_link_libraries(blackgpib
        pico_stdlib pico_fatfs hardware_gpio)

target_include_directories(blackgpib PRIVATE
        ${CMAKE_CURRENT_LIST_DIR})

pico_add_extra_outputs(blackgpib)
