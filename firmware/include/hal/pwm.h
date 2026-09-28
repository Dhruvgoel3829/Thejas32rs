#ifndef HAL_PWM_H
#define HAL_PWM_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

#define PWM_MAX_CHANNELS 8

bool hal_pwm_init(void);
bool hal_pwm_set_us(uint8_t ch, uint16_t us);
bool hal_pwm_set_norm(uint8_t ch, uint16_t norm_0_1000);
bool hal_pwm_arm_outputs(bool arm);
bool hal_pwm_set_freq(uint32_t freq_hz);

#ifdef __cplusplus
}
#endif

#endif /* HAL_PWM_H */