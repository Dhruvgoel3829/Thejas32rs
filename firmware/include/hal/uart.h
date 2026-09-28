#ifndef HAL_UART_H
#define HAL_UART_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    UART_ID_TELEM = 0,
    UART_ID_GNSS = 1,
    UART_ID_RC = 2,
    UART_ID_MAX = 3
} uart_id_t;

bool hal_uart_init(uart_id_t id, uint32_t baud);
bool hal_uart_write_nb(uart_id_t id, const uint8_t *buf, uint16_t len);
bool hal_uart_read(uart_id_t id, uint8_t *buf, uint16_t max_len, uint16_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* HAL_UART_H */