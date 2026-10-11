/*
 * VEGA Pilot Firmware - UART HAL (Emulator Backend)
 * Uses 3x 16550-style UARTs at 0x1000_0000, 0x1000_1000, 0x1000_2000
 */

#include "config.h"
#include "types.h"
#include "hal/uart.h"
#include "hal_vega/regs.h"

#define UART_SIZE 0x1000

static inline uint32_t uart_base(uart_id_t id) {
    switch (id) {
        case UART_ID_TELEM: return UART0_BASE;  // 0x1000_0000
        case UART_ID_GNSS:  return UART1_BASE;  // 0x1000_1000
        case UART_ID_RC:    return UART2_BASE;  // 0x1000_2000
        default:            return UART0_BASE;
    }
}

static inline uint32_t uart_thr(uart_id_t id) { return uart_base(id) + 0x00; }
static inline uint32_t uart_rhr(uart_id_t id) { return uart_base(id) + 0x00; }
static inline uint32_t uart_lsr(uart_id_t id) { return uart_base(id) + 0x05; }

static inline void mmio_write8(uint32_t addr, uint8_t val) {
    *(volatile uint8_t *)addr = val;
}

static inline uint8_t mmio_read8(uint32_t addr) {
    return *(volatile uint8_t *)addr;
}

bool hal_uart_init(uart_id_t id, uint32_t baud) {
    (void)baud;  /* Emulator UART has fixed baud */
    (void)id;
    return true;
}

bool hal_uart_write_nb(uart_id_t id, const uint8_t *buf, uint16_t len) {
    uint32_t thr = uart_thr(id);
    uint32_t lsr = uart_lsr(id);
    for (uint16_t i = 0; i < len; i++) {
        /* Wait for TX ready */
        while ((mmio_read8(lsr) & UART_LSR_TX_READY) == 0);
        mmio_write8(thr, buf[i]);
    }
    return true;
}

bool hal_uart_read(uart_id_t id, uint8_t *buf, uint16_t max_len, uint16_t *out_len) {
    uint32_t rhr = uart_rhr(id);
    uint32_t lsr = uart_lsr(id);
    uint16_t count = 0;
    while (count < max_len) {
        if (mmio_read8(lsr) & UART_LSR_RX_READY) {
            buf[count++] = mmio_read8(rhr);
        } else {
            break;
        }
    }
    *out_len = count;
    return count > 0;
}