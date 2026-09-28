#ifndef HAL_MAG_H
#define HAL_MAG_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int16_t mx, my, mz;
    time_us_t t_us;
} mag_raw_t;

bool hal_mag_init(void);
bool hal_mag_read(mag_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_MAG_H */