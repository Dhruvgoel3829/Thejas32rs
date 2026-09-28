#ifndef HAL_BARO_H
#define HAL_BARO_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    float pressure_pa;
    float temp_c;
    time_us_t t_us;
} baro_raw_t;

bool hal_baro_init(void);
bool hal_baro_read(baro_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_BARO_H */