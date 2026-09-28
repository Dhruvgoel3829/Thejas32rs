#ifndef HAL_IMU_H
#define HAL_IMU_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int16_t ax, ay, az;
    int16_t gx, gy, gz;
    time_us_t t_us;
} imu_raw_t;

bool hal_imu_init(void);
bool hal_imu_read(imu_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_IMU_H */