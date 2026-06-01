#include "img.h"

#include <stdlib.h>

typedef struct {
    FIL file;
} img_loader_t;

bool is_supported_ext(const char* ext) {
    return false;
}

disk_loader_err_t open(FATFS* fs, const char* path, void** this) {
    FRESULT res;

    img_loader_t* loader = malloc(sizeof(img_loader_t));
    if (loader == NULL)
        return LOADER_NOMEM_ERR;

    res = f_open(&loader->file, path, FA_READ | FA_WRITE | FA_OPEN_EXISTING);
    if (res != FR_OK)
        return LOADER_IO_ERR;

    *this = loader;

    return LOADER_OK;
}

disk_loader_err_t geometry(void *this, disk_geometry_t *out) {
    *out = (disk_geometry_t) {
        .sector_count = 40960,
        .sectors_per_track = 20,
        .tracks_per_cylinder = 20,
    };

    return LOADER_OK;
}

disk_loader_err_t read(void *this, uint16_t sector, uint8_t (*out)[SECTOR_SIZE]) {
    img_loader_t *thiz = this;

    FRESULT res;
    UINT br;

    res = f_lseek(&thiz->file, sector * SECTOR_SIZE);
    if (res)
        return LOADER_IO_ERR;

    res = f_read(&thiz->file, out, SECTOR_SIZE, &br);
    if (res)
        return LOADER_IO_ERR;

    if (br != SECTOR_SIZE)
        return LOADER_IO_ERR;

    return LOADER_OK;
}

disk_loader_err_t write(void *this, uint16_t sector, uint8_t (*data)[SECTOR_SIZE]) {
    img_loader_t *thiz = this;

    FRESULT res;
    UINT bw;

    res = f_lseek(&thiz->file, sector * SECTOR_SIZE);
    if (res)
        return LOADER_IO_ERR;

    res = f_write(&thiz->file, data, SECTOR_SIZE, &bw);
    if (res)
        return LOADER_IO_ERR;

    if (bw != SECTOR_SIZE)
        return LOADER_IO_ERR;

    return LOADER_OK;
}

const disk_loader_vtable_t DISK_IMG_LOADER = {
    .is_supported_ext = is_supported_ext,
    .open = open,
    .geometry = geometry,
    .read = read,
    .write = write,
};
