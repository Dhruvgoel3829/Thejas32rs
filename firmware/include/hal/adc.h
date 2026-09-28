#ifndef HAL_ADC_H
#define HAL_ADC_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint16_t batt_mv;
    uint16_t batt_ma;
    uint16_t rssi;
} adc_raw_t;

bool hal_adc_init(void);
bool hal_adc_read(adc_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_ADC_H */