#ifndef HAL_TIME_H
#define HAL_TIME_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

time_us_t hal_time_us(void);
time_ms_t hal_time_ms(void);
cycles_t hal_cycles(void);

/* Init */
void hal_time_init(void);

/* Delay functions */
void hal_delay_us(uint32_t us);
void hal_delay_ms(uint32_t ms);

/* Safe 64-bit time read (handles wrap on RV32) */
static inline time_us_t hal_time_us_safe(void) {
    uint32_t hi, lo, hi2;
    do {
        hi = hal_time_us() >> 32;
        lo = (uint32_t)hal_time_us();
        hi2 = hal_time_us() >> 32;
    } while (hi != hi2);
    return ((time_us_t)hi << 32) | lo;
}

#ifdef __cplusplus
}
#endif

#endif /* HAL_TIME_H */