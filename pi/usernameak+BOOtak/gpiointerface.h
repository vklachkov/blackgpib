#pragma once

#include <stdint.h>

bool GPIO_IF_Init();
bool GPIO_IF_Finish();

enum GPIO_IF_Mode {
    GPIO_IF_INPUT,
    GPIO_IF_OUTPUT,
};

typedef void (*GPIO_IF_Callback)(int gpio, int level, uint32_t tick);

void GPIO_IF_SetMode(uint8_t pin, GPIO_IF_Mode mode);
void GPIO_IF_SetPullup(uint8_t pin);
void GPIO_IF_SetPullDown(uint8_t pin);
void GPIO_IF_SetCallback(uint8_t pin, GPIO_IF_Callback callback);
void GPIO_IF_SetPinState(uint8_t pin, int state);
int GPIO_IF_GetPinState(uint8_t pin);
uint32_t GPIO_IF_USleep(uint32_t micros);

