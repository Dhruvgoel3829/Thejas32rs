/*
 * VEGA Pilot Firmware - Time HAL (Emulator Backend)
 * Uses CLINT mtime and CSR mcycle
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal_vega/regs.h"

/* CLINT base from emulator memory map */
#define CLINT_BASE 0x02000000
#define CLINT_MTIME     (CLINT_BASE + 0xBFF8)
#define CLINT_MTIMEH    (CLINT_BASE + 0xBFFC)

/* CSR read helpers */
static inline uint32_t csr_read_mcycle(void) {
    uint32_t val;
    __asm__ volatile("csrr %0, mcycle" : "=r"(val));
    return val;
}

static inline uint32_t csr_read_mcycleh(void) {
    uint32_t val;
    __asm__ volatile("csrr %0, mcycleh" : "=r"(val));
    return val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

void hal_time_init(void) {
    /* Nothing to init for CLINT/CSR */
}

time_us_t hal_time_us(void) {
    uint32_t hi, lo, hi2;
    do {
        hi = mmio_read32(CLINT_MTIMEH);
        lo = mmio_read32(CLINT_MTIME);
        hi2 = mmio_read32(CLINT_MTIMEH);
    } while (hi != hi2);
    /* mtime ticks once per retired instruction; convert to microseconds
     * via MTIME_HZ (nominal 100 MHz -> 100 ticks/us). Without this scaling,
     * firmware timeouts and emulator busy windows (UART TX, flash program/
     * erase) disagree by 100x. */
    return (((time_us_t)hi << 32) | lo) / (MTIME_HZ / 1000000);
}

time_ms_t hal_time_ms(void) {
    return hal_time_us() / 1000;
}

cycles_t hal_cycles(void) {
    uint32_t hi, lo, hi2;
    do {
        hi = csr_read_mcycleh();
        lo = csr_read_mcycle();
        hi2 = csr_read_mcycleh();
    } while (hi != hi2);
    return ((cycles_t)hi << 32) | lo;
}

void hal_delay_us(uint32_t us) {
    time_us_t start = hal_time_us();
    while ((hal_time_us() - start) < us);
}

void hal_delay_ms(uint32_t ms) {
    hal_delay_us(ms * 1000);
}