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

/* Non-blocking chunked flash operations (cooperative, never blocks).
 * The caller owns `src` until the op completes. Poll once per scheduler
 * tick; returns true when done. Timed phases match the emulator's busy
 * model (QSPI_PAGE_PROGRAM_US / QSPI_SECTOR_ERASE_MS in config.h). */
typedef enum {
    FLASH_OP_IDLE = 0,
    FLASH_OP_ERASING,
    FLASH_OP_PROGRAMMING,
    FLASH_OP_DONE,
    FLASH_OP_ERROR
} flash_op_state_t;

typedef struct {
    flash_op_state_t state;
    uint32_t addr;          /* current program address */
    const uint8_t *src;     /* caller-owned program data */
    uint32_t remaining;
    uint32_t sector;        /* erase target */
    time_us_t wait_until_us;
} flash_op_t;

void flash_op_init(flash_op_t *op);
bool flash_op_start_erase(flash_op_t *op, uint32_t sector);
bool flash_op_start_program(flash_op_t *op, uint32_t addr, const uint8_t *src, uint32_t len);
bool flash_op_poll(flash_op_t *op);

#ifdef __cplusplus
}
#endif

#endif /* HAL_FLASH_H */