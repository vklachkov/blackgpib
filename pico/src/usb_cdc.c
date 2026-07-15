#include "usb_cdc.h"

#include <ctype.h>
#include <stdio.h>
#include <string.h>

static bool copy_token(char* destination, size_t size, const char* token) {
  size_t length = strlen(token);

  if (length == 0 || length >= size) return false;
  memcpy(destination, token, length + 1);
  return true;
}

static bool valid_name(const char* name) {
  if (*name == '\0') return false;
  while (*name) {
    if (!isalnum((unsigned char)*name) && *name != '_') return false;
    name++;
  }
  return true;
}

static bool parse_command(char* line, usb_cdc_command_t* command) {
  char* name;
  char* arguments;
  char* separator;

  if (strncmp(line, "AT+", 3)) return false;

  name = line + 3;
  arguments = strchr(name, '=');
  if (arguments != NULL) {
    *arguments++ = '\0';
    if (!strcmp(arguments, "?")) {
      command->form = USB_CDC_TEST;
      arguments = NULL;
    } else {
      command->form = USB_CDC_SET;
    }
  } else {
    arguments = strchr(name, '?');
    if (arguments != NULL) {
      if (arguments[1] != '\0') return false;
      *arguments = '\0';
      command->form = USB_CDC_QUERY;
    } else {
      command->form = USB_CDC_EXECUTE;
    }
  }
  if (!valid_name(name) || !copy_token(command->name, sizeof(command->name), name)) {
    return false;
  }

  command->argc = 0;
  while (arguments != NULL) {
    separator = strchr(arguments, ',');
    if (separator != NULL) *separator++ = '\0';
    if (command->argc == USB_CDC_COMMAND_ARG_COUNT ||
        !copy_token(command->args[command->argc], USB_CDC_COMMAND_ARG_SIZE, arguments)) {
      return false;
    }
    command->argc++;
    arguments = separator;
  }
  return true;
}

void usb_cdc_read_command(usb_cdc_command_t* command) {
  static char line[USB_CDC_LINE_SIZE];
  static size_t length;

  while (true) {
    int c = getchar();
    if (c < 0) continue;

    if (c == '\r' || c == '\n') {
      line[length] = '\0';
      length = 0;
      if (parse_command(line, command)) return;
    } else if (length < sizeof(line) - 1) {
      line[length++] = (char)c;
    }
  }
}
