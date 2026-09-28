/*
 * VEGA Pilot Firmware - UART HAL (Emulator Backend)
 * Uses 16550-style UART at 0x10000000
 */

#include "config.h"
#include "types.h"
#include "hal/uart.h"
#include "hal_vega/regs.h"

static volatile uint16_t uart_tx_head[UART_ID_MAX] = {0};
static volatile uint16_t uart_tx_tail[UART_ID_MAX] = {0};

static inline void mmio_write8(uint32_t addr, uint8_t val) {
    *(volatile uint8_t *)addr = val;
}

static inline uint8_t mmio_read8(uint32_t addr) {
    return *(volatile uint8_t *)addr;
}

bool hal_uart_init(uart_id_t id, uint32_t baud) {
    (void)baud;  /* Emulator UART has fixed baud */
    (void)id;    /* Emulator has single UART */
    return true;
}

bool hal_uart_write_nb(uart_id_t id, const uint8_t *buf, uint16_t len) {
    (void)id;
    for (uint16_t i = 0; i < len; i++) {
        /* Wait for TX ready */
        while ((mmio_read8(UART_LSR) & UART_LSR_TX_READY) == 0);
        mmio_write8(UART_THR, buf[i]);
    }
    return true;
}

bool hal_uart_read(uart_id_t id, uint8_t *buf, uint16_t max_len, uint16_t *out_len) {
    (void)id;
    uint16_t count = 0;
    while (count < max_len) {
        if (mmio_read8(UART_LSR) & UART_LSR_RX_READY) {
            buf[count++] = mmio_read8(UART_RHR);
        } else {
            break;
        }
    }
    *out_len = count;
    return count > 0;
}