#ifndef HAL_TIME_HOST_H
#define HAL_TIME_HOST_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Test helper functions for host time mock */
void hal_time_mock_advance_us(uint32_t us);
void hal_time_mock_set_cycles(uint64_t cycles);

#ifdef __cplusplus
}
#endif

#endif /* HAL_TIME_HOST_H */