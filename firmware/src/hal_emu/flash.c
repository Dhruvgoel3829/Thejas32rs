/*
 * VEGA Pilot Firmware - Flash HAL (Emulator Backend)
 * QSPI Flash with file-backed persistence
 */

#include "config.h"
#include "types.h"
#include "hal/flash.h"
#include "hal/time.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_flash_init(void) {
    /* Nothing to init for emulator flash */
    return true;
}

bool hal_flash_read(uint32_t addr, uint8_t *buf, uint32_t len) {
    for (uint32_t i = 0; i < len; i++) {
        buf[i] = *(volatile uint8_t *)(addr + i);
    }
    return true;
}

bool hal_flash_write(uint32_t addr, const uint8_t *buf, uint32_t len) {
    for (uint32_t i = 0; i < len; i++) {
        *(volatile uint8_t *)(addr + i) = buf[i];
    }
    return true;
}

bool hal_flash_erase_sector(uint32_t sector) {
    uint32_t start = sector * 4096;
    for (uint32_t i = 0; i < 4096; i++) {
        *(volatile uint8_t *)(FLASH_BASE + start + i) = 0xFF;
    }
    return true;
}

bool hal_flash_erase_range(uint32_t addr, uint32_t len) {
    for (uint32_t i = 0; i < len; i++) {
        *(volatile uint8_t *)(addr + i) = 0xFF;
    }
    return true;
}

void flash_op_init(flash_op_t *op) {
    op->state = FLASH_OP_IDLE;
    op->addr = 0;
    op->src = NULL;
    op->remaining = 0;
    op->sector = 0;
    op->wait_until_us = 0;
}

bool flash_op_start_erase(flash_op_t *op, uint32_t sector) {
    if (op->state == FLASH_OP_ERASING || op->state == FLASH_OP_PROGRAMMING) {
        return false;  /* op already in flight */
    }
    op->state = FLASH_OP_ERASING;
    op->sector = sector;
    op->wait_until_us = 0;  /* erase issues immediately on first poll */
    return true;
}

bool flash_op_start_program(flash_op_t *op, uint32_t addr, const uint8_t *src, uint32_t len) {
    if (op->state == FLASH_OP_ERASING || op->state == FLASH_OP_PROGRAMMING) {
        return false;  /* op already in flight */
    }
    if (src == NULL && len > 0) {
        op->state = FLASH_OP_ERROR;
        return false;
    }
    op->state = FLASH_OP_PROGRAMMING;
    op->addr = addr;
    op->src = src;
    op->remaining = len;
    op->wait_until_us = 0;  /* first page programs immediately on first poll */
    return true;
}

bool flash_op_poll(flash_op_t *op) {
    time_us_t now = hal_time_us();

    switch (op->state) {
        case FLASH_OP_IDLE:
        case FLASH_OP_DONE:
        case FLASH_OP_ERROR:
            return op->state == FLASH_OP_DONE;

        case FLASH_OP_ERASING: {
            if (now < op->wait_until_us) {
                return false;  /* yield: erase still busy */
            }
            if (op->wait_until_us == 0) {
                /* Issue the erase, then wait out the busy window */
                hal_flash_erase_sector(op->sector);
                op->wait_until_us = now + (time_us_t)QSPI_SECTOR_ERASE_MS * 1000;
                return false;
            }
            op->state = FLASH_OP_DONE;
            return true;
        }

        case FLASH_OP_PROGRAMMING: {
            if (op->remaining == 0) {
                op->state = FLASH_OP_DONE;
                return true;
            }
            if (now < op->wait_until_us) {
                return false;  /* yield: program still busy */
            }
            /* Program one page (or remainder) per poll */
            uint32_t chunk = op->remaining;
            if (chunk > QSPI_PAGE_SIZE) {
                chunk = QSPI_PAGE_SIZE;
            }
            hal_flash_write(op->addr, op->src, chunk);
            op->addr += chunk;
            op->src += chunk;
            op->remaining -= chunk;
            op->wait_until_us = now + QSPI_PAGE_PROGRAM_US;
            return op->remaining == 0;
        }
    }
    return false;
}