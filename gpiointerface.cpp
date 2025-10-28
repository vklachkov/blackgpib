#include "gpiointerface.h"
#include <pigpio.h>

bool GPIO_IF_Init() {
    return gpioInitialise() >= 0;
}

bool GPIO_IF_Finish() {
    gpioTerminate();
    return true;
}

void GPIO_IF_SetMode(uint8_t pin, GPIO_IF_Mode mode) {
    if(mode == GPIO_IF_INPUT) {
        gpioSetMode(pin, PI_INPUT);
    } else if (mode == GPIO_IF_OUTPUT) {
        gpioSetMode(pin, PI_OUTPUT);
    }
}

void GPIO_IF_SetPullDown(uint8_t pin) {
    gpioSetPullUpDown(pin, PI_PUD_DOWN);
}

void GPIO_IF_SetPullup(uint8_t pin) {
    gpioSetPullUpDown(pin, PI_PUD_UP);
}

void GPIO_IF_SetCallback(uint8_t pin, GPIO_IF_Callback callback) {
    gpioSetAlertFunc(pin, callback);
}

void GPIO_IF_SetPinState(uint8_t pin, int state) {
    gpioWrite(pin, state);
}

int GPIO_IF_GetPinState(uint8_t pin) {
    return gpioRead(pin);
}

uint32_t GPIO_IF_USleep(uint32_t micros) {
    return gpioDelay(micros);
}

