/*
 * VEGA Pilot Firmware - RC HAL (Emulator Backend)
 * SBUS/CRSF/PPM parser over UART
 */

#include "config.h"
#include "types.h"
#include "hal/rc.h"
#include "hal/uart.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_rc_init(void) {
    /* Use UART_ID_RC for RC receiver */
    hal_uart_init(UART_ID_RC, 100000);  /* SBUS/CRSF baud */
    return true;
}

bool hal_rc_read(rc_raw_t *out) {
    /* TODO: Parse SBUS/CRSF from UART */
    (void)out;
    return false;
}

void rc_parser_feed(rc_rx_t *rx, uint8_t byte) {
    (void)rx;
    (void)byte;
}

bool rc_parser_parse(rc_rx_t *rx, rc_raw_t *out) {
    (void)rx;
    (void)out;
    return false;
}