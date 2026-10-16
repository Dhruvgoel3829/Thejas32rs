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

static inline bool qspi_busy(void) {
    return (mmio_read32(QSPI_STATUS) & QSPI_BUSY) != 0;
}

static void qspi_issue_erase(uint32_t sector) {
    mmio_write32(QSPI_ARG, sector);
    mmio_write32(QSPI_CMD, QSPI_CMD_ERASE_SECTOR);
}

bool hal_flash_write(uint32_t addr, const uint8_t *buf, uint32_t len) {
    /* Blocking program via single-byte MMIO writes, which the controller
     * models as instant (busy applies to multi-byte ops and erases).
     * Slow path only if a background op left the controller busy. */
    for (uint32_t i = 0; i < len; i++) {
        while (qspi_busy()) {
        }
        *(volatile uint8_t *)(addr + i) = buf[i];
    }
    return true;
}

bool hal_flash_erase_sector(uint32_t sector) {
    qspi_issue_erase(sector);
    while (qspi_busy()) {
    }
    return true;
}

bool hal_flash_erase_range(uint32_t addr, uint32_t len) {
    if (len == 0) {
        return true;
    }
    /* Erase every 4 KB sector overlapped by [addr, addr+len) */
    uint32_t first = (addr - FLASH_BASE) / QSPI_SECTOR_SIZE;
    uint32_t last = (addr - FLASH_BASE + len - 1) / QSPI_SECTOR_SIZE;
    for (uint32_t s = first; s <= last; s++) {
        if (!hal_flash_erase_sector(s)) {
            return false;
        }
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
            if (op->wait_until_us == 0) {
                /* Issue the erase (non-blocking), then poll status below */
                qspi_issue_erase(op->sector);
                op->wait_until_us = now + 2u * (time_us_t)QSPI_SECTOR_ERASE_MS * 1000u;
                return false;
            }
            if (qspi_busy()) {
                /* Yield while busy; deadline guards a stuck controller */
                if (now >= op->wait_until_us) {
                    op->state = FLASH_OP_ERROR;
                }
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
            if (qspi_busy()) {
                /* Yield while busy; deadline guards a stuck controller */
                if (now >= op->wait_until_us) {
                    op->state = FLASH_OP_ERROR;
                }
                return false;
            }
            /* Program one page (or remainder) per poll via raw MMIO;
             * the controller sets busy, which the next poll observes. */
            uint32_t chunk = op->remaining;
            if (chunk > QSPI_PAGE_SIZE) {
                chunk = QSPI_PAGE_SIZE;
            }
            for (uint32_t i = 0; i < chunk; i++) {
                *(volatile uint8_t *)(op->addr + i) = op->src[i];
            }
            op->addr += chunk;
            op->src += chunk;
            op->remaining -= chunk;
            op->wait_until_us = hal_time_us() + 2u * (time_us_t)QSPI_PAGE_PROGRAM_US;
            return op->remaining == 0;
        }
    }
    return false;
}