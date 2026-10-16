#ifndef HAL_I2C_H
#define HAL_I2C_H

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

typedef struct {
    float pressure_pa;
    float temp_c;
    time_us_t t_us;
} baro_raw_t;

bool hal_baro_init(void);
bool hal_baro_read(baro_raw_t *out);

// I2C master functions
bool hal_i2c_init(void);
bool hal_i2c_write(uint8_t addr, uint8_t reg, uint8_t len, const uint8_t *data);
bool hal_i2c_read(uint8_t addr, uint8_t reg, uint8_t len, uint8_t *data);

#ifdef __cplusplus
}
#endif

#endif /* HAL_I2C_H */