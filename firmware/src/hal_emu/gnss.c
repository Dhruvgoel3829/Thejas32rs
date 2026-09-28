/*
 * VEGA Pilot Firmware - GNSS HAL (Emulator Backend)
 * NMEA parser over UART
 */

#include "config.h"
#include "types.h"
#include "hal/gnss.h"
#include "hal/uart.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_gnss_init(void) {
    /* Use UART_ID_GNSS for GPS NMEA */
    hal_uart_init(UART_ID_GNSS, 9600);
    return true;
}

bool hal_gnss_read(gnss_raw_t *out) {
    /* TODO: Parse NMEA from UART */
    (void)out;
    return false;
}

void nmea_parser_feed(nmea_rx_t *rx, uint8_t byte) {
    (void)rx;
    (void)byte;
}

bool nmea_parser_parse(nmea_rx_t *rx, gnss_raw_t *out) {
    (void)rx;
    (void)out;
    return false;
}