#pragma once

#include <stdbool.h>
#include <stddef.h>

#define USB_CDC_LINE_SIZE          128
#define USB_CDC_COMMAND_NAME_SIZE  16
#define USB_CDC_COMMAND_ARG_COUNT  4
#define USB_CDC_COMMAND_ARG_SIZE   16

typedef enum {
  USB_CDC_EXECUTE,
  USB_CDC_QUERY,
  USB_CDC_TEST,
  USB_CDC_SET,
} usb_cdc_command_form_t;

typedef struct {
  char name[USB_CDC_COMMAND_NAME_SIZE];
  char args[USB_CDC_COMMAND_ARG_COUNT][USB_CDC_COMMAND_ARG_SIZE];
  size_t argc;
  usb_cdc_command_form_t form;
} usb_cdc_command_t;

void usb_cdc_read_command(usb_cdc_command_t* command);
