#include "gpiointerface.h"

#include <stdio.h>
#include <cstring>
#include <vector>
#include <queue>
#include <memory>
#include <unistd.h>

#define GRID210X_GPIB_STATE_IDLE 0
#define GRID210X_GPIB_STATE_WAIT_DAV_FALSE 1
#define GRID210X_GPIB_STATE_SEND_DATA_START 2
#define GRID210X_GPIB_STATE_WAIT_NDAC_FALSE 3
#define GRID210X_GPIB_STATE_WAIT_ATN_UNASSERT 4

#define GRID210X_STATE_IDLE 0
#define GRID210X_STATE_READING_DATA 1
#define GRID210X_STATE_WRITING_DATA 2
#define GRID210X_STATE_WRITING_DATA_WAIT 3

#define SILENT 1

#define LOG(format, args...)

// #define LOG_INFO(format, args...)

#define LOG_INFO(format, args...) { \
	timespec ts; \
	clock_gettime(CLOCK_MONOTONIC, &ts); \
	printf("+%lu.%09lu: " format, ts.tv_sec, ts.tv_nsec, ##args); \
} \

#define GRID2102_FETCH32(Array, Offset) ((uint32_t)(\
	(Array[Offset] << 0) |\
	(Array[Offset + 1] << 8) |\
	(Array[Offset + 2] << 16) |\
	(Array[Offset + 3] << 24)\
))

#define GRID2102_FETCH16(Array, Offset) ((uint16_t)(\
	(Array[Offset] << 0) |\
	(Array[Offset + 1] << 8)\
))

typedef void (*LineCallback)();

struct LineCallbacks {
	LineCallback onATNLow;
	LineCallback onATNHigh;
	LineCallback onDAVLow;
	LineCallback onDAVHigh;
	LineCallback onNRFDLow;
	LineCallback onNRFDHigh;
	LineCallback onNDACLow;
	LineCallback onNDACHigh;
};

LineCallback stateMachine = {0};

uint8_t G210x_gpib_state = GRID210X_GPIB_STATE_IDLE;
uint8_t G210x_state = GRID210X_STATE_IDLE;
uint8_t G210x_last_recv_byte;
int G210x_last_recv_eoi;
int G210x_last_recv_atn;
bool G210x_listening = false;
bool G210x_talking = false;
bool G210x_serial_polling = false;
// int G210x_bus_addr = 4;
int G210x_bus_addr = 5;
std::vector<uint8_t> G210x_data_buffer;
std::queue<uint8_t> G210x_output_data_buffer;
bool G210x_has_srq = false;
uint8_t G210x_byte_to_send = 0;
uint8_t G210x_serial_poll_byte = 0;
uint8_t G210x_send_eoi = 0;
uint32_t G210x_floppy_sector_number = 0;
uint16_t G210x_io_size = 0;
FILE *G210x_image_file = NULL;
useconds_t G210x_read_delay = 150;


// uint8_t G2101H_identify_response[56] = {
// 	0x00, 0x02, 0xF8, 0x01, 0x8C, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x4D, 0x41,
// 	0x4D, 0x45, 0x20, 0x48, 0x41, 0x52, 0x44, 0x44, 0x49, 0x53, 0x4B, 0x20, 0x44, 0x52, 0x49, 0x56,
// 	0x45, 0x20, 0x20, 0x20, 0x20, 0x20, 0x47, 0x52, 0x49, 0x44, 0x32, 0x31, 0x30, 0x31, 0x00, 0x02,
// 	0x11, 0x00, 0x33, 0x01, 0x00, 0x00, 0x04, 0x00
// };

uint8_t G2101H_identify_response[52] = {
	0x00, 0x02, 0xF8, 0x01, 0xD0, 0x02, 0x01, 0x20, 0x01, 0x21, 0x01, 0x01, 0x00, 0x00, 0x34, 0x38,
	0x20, 0x54, 0x50, 0x49, 0x20, 0x44, 0x53, 0x20, 0x44, 0x44, 0x20, 0x46, 0x4C, 0x4F, 0x50, 0x50,
	0x59, 0x20, 0x20, 0x20, 0x20, 0x33, 0x30, 0x30, 0x32, 0x33, 0x37, 0x2D, 0x30, 0x30, 0x00, 0x02,
	0x09, 0x00, 0x02, 0x00
};

enum GPIB_Pin {
    GPIB_PIN_DIO1,
    GPIB_PIN_DIO2,
    GPIB_PIN_DIO3,
    GPIB_PIN_DIO4,
    GPIB_PIN_DIO5,
    GPIB_PIN_DIO6,
    GPIB_PIN_DIO7,
    GPIB_PIN_DIO8,

    GPIB_PIN_EOI,
    GPIB_PIN_ATN,
    GPIB_PIN_DAV,
    GPIB_PIN_NRFD,
    GPIB_PIN_NDAC,

    GPIB_PIN_SRQ,

    GPIB_PIN_MAX
};

uint8_t remapGPIBPin(GPIB_Pin pin) {
    switch(pin) {
        case GPIB_PIN_DIO1: return 4;
        case GPIB_PIN_DIO2: return 17;
        case GPIB_PIN_DIO3: return 27;
        case GPIB_PIN_DIO4: return 22;
        case GPIB_PIN_DIO5: return 18;
        case GPIB_PIN_DIO6: return 23;
        case GPIB_PIN_DIO7: return 24;
        case GPIB_PIN_DIO8: return 25;

        case GPIB_PIN_EOI: return 26;
        case GPIB_PIN_DAV: return 19;
        case GPIB_PIN_NRFD: return 5;
        case GPIB_PIN_NDAC: return 6;
        case GPIB_PIN_ATN: return 12;

        case GPIB_PIN_SRQ: return 13;
    }
    return 0xFF;
}

int GPIB_host_pin_states[GPIB_PIN_MAX] {};
// int GPIB_pin_states[GPIB_PIN_MAX] {};

#define GPIB_WRITE_FUNCTION_DEFINE(pinName) \
    void GPIB_Write_##pinName(int value) { \
		LOG("Write " #pinName" %d\n", value); \
        GPIB_Pin gpibPin = GPIB_PIN_##pinName; \
        /*if (GPIB_pin_states[gpibPin] == value) return;*/ \
        uint8_t pin = remapGPIBPin(gpibPin); \
        if (value) { \
            GPIO_IF_SetMode(pin, GPIO_IF_INPUT); \
        } else { \
            GPIO_IF_SetMode(pin, GPIO_IF_OUTPUT); \
	        GPIO_IF_SetPinState(pin, value); \
        } \
        /*GPIB_pin_states[gpibPin] = value;*/ \
    }

int GPIB_Read_Pin(GPIB_Pin gpibPin) {
	// if (GPIB_pin_states[gpibPin] == 0) return 0;
	return GPIB_host_pin_states[gpibPin];
}

#define GPIB_READ_FUNCTION_DEFINE(pinName) \
    int GPIB_Read_##pinName() { \
        return GPIB_Read_Pin(GPIB_PIN_##pinName); \
    }

GPIB_WRITE_FUNCTION_DEFINE(NDAC)
GPIB_WRITE_FUNCTION_DEFINE(SRQ)
GPIB_WRITE_FUNCTION_DEFINE(EOI)
GPIB_WRITE_FUNCTION_DEFINE(DAV)
GPIB_WRITE_FUNCTION_DEFINE(NRFD)

// bool GPIO_passive(uint8_t pin) {
//     gpioSetMode(pin, PI_INPUT);
//     gpioSetPullUpDown(pin, PI_PUD_UP);
//     return (bool)gpioRead(pin);
// }

// void GPIO_active(uint8_t pin) {
//     gpioSetMode(pin, PI_OUTPUT);
//     gpioWrite(pin, state);
// }

void GPIB_Write_DIO(int byteValue) {
    for(int i = 0; i < 8; i++) {
        int value = (byteValue >> i) & 1;

        GPIB_Pin gpibPin = (GPIB_Pin) (GPIB_PIN_DIO1 + i);
        uint8_t pin = remapGPIBPin(gpibPin);
		GPIO_IF_SetMode(pin, GPIO_IF_OUTPUT);
        GPIO_IF_SetPinState(pin, value);
    }
}

GPIB_READ_FUNCTION_DEFINE(ATN)
GPIB_READ_FUNCTION_DEFINE(EOI)
GPIB_READ_FUNCTION_DEFINE(NDAC)
GPIB_READ_FUNCTION_DEFINE(NRFD)

int GPIB_Read_DIO() {
    int byteValue = 0;
    for(int i = 0; i < 8; i++) {
		GPIB_Pin gpibPin = (GPIB_Pin) (GPIB_PIN_DIO1 + i);
        uint8_t pin = remapGPIBPin(gpibPin);
		GPIO_IF_SetMode(pin, GPIO_IF_INPUT);
		GPIO_IF_SetPullDown(pin);
        // GPIB_Pin gpibPin = (GPIB_Pin) (GPIB_PIN_DIO1 + i);
        byteValue |= GPIO_IF_GetPinState(pin) << i;
    }
    return byteValue;
}

void G210x_ExecTimer() {
	if (G210x_state == GRID210X_STATE_READING_DATA) {
		std::unique_ptr<uint8_t[]> data(new uint8_t[G210x_io_size]);
		fseek(G210x_image_file, G210x_floppy_sector_number * 512, SEEK_SET);
		fread(data.get(), 1, G210x_io_size, G210x_image_file);
		for (int i = 0; i < G210x_io_size; i++) {
			G210x_output_data_buffer.push(data[i]);
		}
		G210x_serial_poll_byte = 0x0F;
		G210x_has_srq = true;
		GPIB_Write_SRQ(0);
		G210x_state = GRID210X_STATE_IDLE;
	} else if (G210x_state == GRID210X_STATE_WRITING_DATA_WAIT) {
		// send an srq as success flag
		for (int i = 0; i < 7; i++) { // FIXME:
			G210x_output_data_buffer.push(0);
		}
		G210x_serial_poll_byte = 0x0F;
		G210x_has_srq = true;
		GPIB_Write_SRQ(0);
		G210x_state = GRID210X_STATE_IDLE;
	}
}

void G210x_AcceptTransfer() {
    if (G210x_state == GRID210X_STATE_IDLE) {
		if (G210x_data_buffer.size() >= 0xA) {
			uint8_t command = G210x_data_buffer[0];
			uint32_t sector_number = GRID2102_FETCH32(G210x_data_buffer, 3);
			uint16_t data_size = GRID2102_FETCH16(G210x_data_buffer, 7);
			LOG_INFO("grid210x_device command %u, data size %u, sector no %u\n", (unsigned)command, (unsigned)data_size, (unsigned)sector_number);
			(void)(sector_number);
			if (command == 0x1) { // ddGetStatus
				for (int i = 0; i < 52 && i < data_size; i++) {
					G210x_output_data_buffer.push(G2101H_identify_response[i]);
				}
			} else if (command == 0x4) { // ddRead
				G210x_floppy_sector_number = sector_number;
				G210x_io_size = data_size;
				G210x_state = GRID210X_STATE_READING_DATA;
                // usleep(G210x_read_delay);
				G210x_ExecTimer();
			} else if (command == 0x5) {
				G210x_floppy_sector_number = sector_number;
				G210x_io_size = data_size;
				G210x_state = GRID210X_STATE_WRITING_DATA;
			}
		} // else something is wrong, ignore
	} else if (G210x_state == GRID210X_STATE_WRITING_DATA) {
		// write
		if (G210x_floppy_sector_number != 0xFFFFFFFF) {
			fseek(G210x_image_file, G210x_floppy_sector_number * 512, SEEK_SET);
			fwrite(G210x_data_buffer.data(), 1, G210x_data_buffer.size(), G210x_image_file);
		} else {
			// TODO: set status
		}
		LOG("grid210x_device write sector %d\n", floppy_sector_number);
		// wait
		G210x_state = GRID210X_STATE_WRITING_DATA_WAIT;
        // usleep(G210x_read_delay);
        G210x_ExecTimer();
	}
}

void G210x_UpdateNDAC(int atn) {
	if (G210x_gpib_state == GRID210X_GPIB_STATE_IDLE) {
		if (atn) {
			// pull NDAC low
			GPIB_Write_NDAC(0);
		} else {
			// pull NDAC high if not listener and low if listener
			GPIB_Write_NDAC(G210x_listening ? 0 : 1);
		}
	}
}

void G210x_NDAC_Callback(int state);

void G210x_NRFD_Callback(int state)  {
	if (state == 0 || state == 1) {
		LOG("receive NRFD=%d\n", state);
	}
	if (state == 1 && G210x_gpib_state == GRID210X_GPIB_STATE_SEND_DATA_START) {
		// set dio and assert dav
		GPIB_Write_DIO(G210x_byte_to_send ^ 0xFF);
		GPIB_Write_EOI(G210x_send_eoi ^ 1);

		// GPIO_IF_USleep(10);

		LOG_INFO("grid210x_device byte send %02x eoi %d\n", G210x_byte_to_send, G210x_send_eoi);
		GPIB_Write_DAV(0);
		GPIB_Write_NDAC(1);
		G210x_gpib_state = GRID210X_GPIB_STATE_WAIT_NDAC_FALSE;
		G210x_NDAC_Callback(GPIB_Read_NDAC());
	}
	// LOG("grid210x_device nrfd state set to %d\n", state);
}

void G210x_NDAC_Callback(int state)  {
	if (state == 0 || state == 1) {
		LOG("receive NDAC=%d\n", state);
	}
	if (state == 1 && G210x_gpib_state == GRID210X_GPIB_STATE_WAIT_NDAC_FALSE) {
		// restore initial state
		// LOG("grid210x_device restore ndac nrfd dav eoi\n");

		// GPIO_IF_USleep(5);

		GPIB_Write_DAV(1);
		GPIB_Write_EOI(1);

		// GPIO_IF_USleep(10);
 
		GPIB_Write_NRFD(1);
        
		G210x_gpib_state = GRID210X_GPIB_STATE_IDLE;
		if (G210x_serial_polling) {
			G210x_talking = false;
			GPIB_Write_NDAC(0);
		} else {
			G210x_UpdateNDAC(GPIB_Read_ATN() ^ 1);
		}

		if (!G210x_serial_polling && G210x_talking && !G210x_output_data_buffer.empty()) {
			G210x_byte_to_send = G210x_output_data_buffer.front();
			G210x_output_data_buffer.pop();
			G210x_send_eoi = G210x_output_data_buffer.empty() ? 1 : 0;
			G210x_gpib_state = GRID210X_GPIB_STATE_SEND_DATA_START;
		} else {
        	// rpi drive emulator fix: CLEAR THE DIO LINES
			GPIB_Write_DIO(0xFF);
		}
	}
	// LOG("grid210x_device ndac state set to %d\n", state);
}

void G210x_ATN_Callback(int state) {
	if (state == 0 || state == 1) {
		LOG("receive ATN=%d\n", state);
	}
	if (state == 1 && G210x_gpib_state == GRID210X_GPIB_STATE_WAIT_ATN_UNASSERT) {
		G210x_gpib_state = GRID210X_GPIB_STATE_SEND_DATA_START;
		G210x_NRFD_Callback(GPIB_Read_NRFD());
	}
    if (state == 1 || state == 0) {
        G210x_UpdateNDAC(state ^ 1);
    }
}

void G210x_DAV_Callback(int state) {
	if (state == 0 || state == 1) {
		LOG("receive DAV=%d\n", state);
	}
    if(state == 0 && G210x_gpib_state == GRID210X_GPIB_STATE_IDLE) {
		// read data and wait for transfer end
		GPIB_Write_NRFD(0);
		int atn = GPIB_Read_ATN() ^ 1;
		uint8_t data = GPIB_Read_DIO() ^ 0xFF;
		int eoi = GPIB_Read_EOI() ^ 1;
		LOG_INFO("grid210x_device byte recv %02x atn %d eoi %d\n", data, atn, eoi);
		G210x_last_recv_byte = data;
		G210x_last_recv_atn = atn;

		if (!atn) {
			// update EOI only in data mode (ATN unasserted)
			G210x_last_recv_eoi = eoi;
		}
		GPIB_Write_NDAC(1);
		G210x_gpib_state = GRID210X_GPIB_STATE_WAIT_DAV_FALSE;
		// GPIO_IF_USleep(100);
	} else if (state == 1 && G210x_gpib_state == GRID210X_GPIB_STATE_WAIT_DAV_FALSE) {
		// restore initial state
		// m_bus->ndac_w(this, 0);
		G210x_gpib_state = GRID210X_GPIB_STATE_IDLE;
		// use ATN value on low DAV; here it can be new already
		G210x_UpdateNDAC(G210x_last_recv_atn);

		// Unassert NRFD early
		GPIB_Write_NRFD(1);

		if (G210x_last_recv_atn) {
			if ((G210x_last_recv_byte & 0xE0) == 0x20) {
				if ((G210x_last_recv_byte & 0x1F) == G210x_bus_addr) {
					// dev-id = 5
					G210x_listening = true;
					LOG_INFO("grid210x_device now listening\n");

					G210x_data_buffer.clear();
				} else if((G210x_last_recv_byte & 0x1F) == 0x1F) {
					// reset listen
					G210x_listening = false;
					LOG_INFO("grid210x_device now not listening\n");

					// Not listening: It's time to handle received command!
					if (G210x_last_recv_eoi) {
						G210x_AcceptTransfer();
						G210x_data_buffer.clear();
					}
				}
			} else if ((G210x_last_recv_byte & 0xE0) == 0x40) {
				if ((G210x_last_recv_byte & 0x1F) == G210x_bus_addr) {
					// dev-id = 5
					G210x_talking = true;
					LOG_INFO("grid210x_device now talking\n");
				} else {
					// reset talk
					G210x_talking = false;
					LOG_INFO("grid210x_device now not talking\n");
				}
			} else if (G210x_last_recv_byte == 0x18) {
				// serial poll enable
				G210x_serial_polling = true;
			} else if (G210x_last_recv_byte == 0x19) {
				// serial poll disable
				G210x_serial_polling = false;
			}
		} else if (G210x_listening) {
			G210x_data_buffer.push_back(G210x_last_recv_byte);
		}

		if (G210x_talking) {
			if (G210x_serial_polling) {
				bool had_srq = G210x_has_srq;
				if (G210x_has_srq) {
					G210x_has_srq = false;
					GPIB_Write_SRQ(1);
				}
				G210x_byte_to_send = G210x_serial_poll_byte | (had_srq ? 0x40 : 0);
				G210x_serial_poll_byte = 0;
				G210x_send_eoi = 0;
				G210x_gpib_state = GRID210X_GPIB_STATE_SEND_DATA_START;
			} else if (!G210x_output_data_buffer.empty()) {
				G210x_byte_to_send = G210x_output_data_buffer.front();
				G210x_output_data_buffer.pop();
				G210x_send_eoi = G210x_output_data_buffer.empty() ? 1 : 0;
				G210x_gpib_state = GRID210X_GPIB_STATE_WAIT_ATN_UNASSERT;
			}
		}

		// GPIO_IF_USleep(100);

		// G210x_NRFD_Callback(GPIB_Read_NRFD());
	}
}

int main(int argc, char **argv) {
    if(argc != 2) {
        fprintf(stderr, "usage: g210x <filename.img>\n");
        return 2;
    }

    if(!GPIO_IF_Init()) {
        fprintf(stderr, "Failed to initialize GPIO\n");
        return 1;
    }

    G210x_image_file = fopen(argv[1], "r+b");
    if(!G210x_image_file) {
        fprintf(stderr, "Failed to open image file.\n");
        return 3;
    }

    // for (int i = 0; i < GPIB_PIN_MAX; i++) {
	// 	uint8_t iopin = remapGPIBPin((GPIB_Pin) i);
	// 	GPIO_IF_SetPullDown(iopin);
	// }

	// usleep(100);

	// for (int i = 0; i < GPIB_PIN_MAX; i++) {
	// 	uint8_t iopin = remapGPIBPin((GPIB_Pin) i);
	// 	fprintf(stderr, "%d:%d %d\n", i, iopin, GPIO_IF_GetPinState(iopin));
	// }

    for (int i = 0; i < GPIB_PIN_MAX; i++) {
        uint8_t iopin = remapGPIBPin((GPIB_Pin) i);
        GPIO_IF_SetPullup(iopin);
        GPIO_IF_SetMode(iopin, GPIO_IF_INPUT);
        // GPIB_pin_states[i] = -1;
        GPIB_host_pin_states[i] = 1;
    }

	while (1) {
		int old_host_pin_states[GPIB_PIN_MAX];
		memcpy(old_host_pin_states, GPIB_host_pin_states, sizeof(GPIB_host_pin_states));

		// check signalling only
		for (int i = GPIB_PIN_DIO8 + 1; i < GPIB_PIN_MAX; i++) {
			GPIB_host_pin_states[i] = /* GPIB_pin_states[i] && */ GPIO_IF_GetPinState(remapGPIBPin((GPIB_Pin) i));
		}

		for (int i = GPIB_PIN_DIO8 + 1; i < GPIB_PIN_MAX; i++) {
			bool changed = false;
			int newState = GPIB_host_pin_states[i];
			if (newState != old_host_pin_states[i]) {
				changed = true;
			}
			if (changed) {
				if (i == GPIB_PIN_ATN) {
					G210x_ATN_Callback(newState);
				} else if (i == GPIB_PIN_DAV) {
					G210x_DAV_Callback(newState);
				} else if (i == GPIB_PIN_NRFD) {
					G210x_NRFD_Callback(newState);
				} else if (i == GPIB_PIN_NDAC) {
					G210x_NDAC_Callback(newState);
				}
			}
		}

		// GPIO_IF_USleep(100);
	}

    GPIO_IF_Finish();

    return 0;
}
