#ifndef HAL_FLASH_H
#define HAL_FLASH_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

#define FLASH_SECTOR_SIZE 4096
#define FLASH_PARAM_SECTOR 0
#define FLASH_LOG_SECTOR_START 1

bool hal_flash_init(void);
bool hal_flash_read(uint32_t addr, uint8_t *buf, uint32_t len);
bool hal_flash_write(uint32_t addr, const uint8_t *buf, uint32_t len);
bool hal_flash_erase_sector(uint32_t sector);
bool hal_flash_erase_range(uint32_t addr, uint32_t len);

#ifdef __cplusplus
}
#endif

#endif /* HAL_FLASH_H */