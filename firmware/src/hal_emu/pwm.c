/*
 * VEGA Pilot Firmware - PWM HAL (Emulator Backend)
 * ESC outputs
 */

#include "config.h"
#include "types.h"
#include "hal/pwm.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_pwm_init(void) {
    /* Default 50 Hz for standard ESCs, or 400 Hz for DShot */
    mmio_write32(PWM_FREQ, 50);
    mmio_write32(PWM_CTRL, PWM_CTRL_EN);
    return true;
}

bool hal_pwm_set_us(uint8_t ch, uint16_t us) {
    if (ch >= PWM_MAX_CHANNELS) return false;
    /* Convert us to duty cycles (12-bit, 0-4095) */
    /* For 50 Hz: period = 20000 us, duty = us / 20000 * 4095 */
    uint32_t duty = (us * 4095) / 20000;
    if (duty > 4095) duty = 4095;
    mmio_write32(PWM_DUTY(ch), duty);
    return true;
}

bool hal_pwm_set_norm(uint8_t ch, uint16_t norm_0_1000) {
    if (ch >= PWM_MAX_CHANNELS) return false;
    if (norm_0_1000 > 1000) norm_0_1000 = 1000;
    uint32_t duty = (norm_0_1000 * 4095) / 1000;
    mmio_write32(PWM_DUTY(ch), duty);
    return true;
}

bool hal_pwm_arm_outputs(bool arm) {
    uint32_t ctrl = mmio_read32(PWM_CTRL);
    if (arm) {
        ctrl |= PWM_CTRL_EN;
        for (int i = 0; i < PWM_MAX_CHANNELS; i++) {
            ctrl |= PWM_CTRL_CH_EN(i);
        }
    } else {
        ctrl &= ~PWM_CTRL_EN;
    }
    mmio_write32(PWM_CTRL, ctrl);
    return true;
}

bool hal_pwm_set_freq(uint32_t freq_hz) {
    if (freq_hz == 0) freq_hz = 1;
    mmio_write32(PWM_FREQ, freq_hz);
    return true;
}