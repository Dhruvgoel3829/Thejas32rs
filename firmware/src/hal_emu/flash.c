/*
 * VEGA Pilot Firmware - Flash HAL (Emulator Backend)
 * QSPI Flash with file-backed persistence
 */

#include "config.h"
#include "types.h"
#include "hal/flash.h"
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