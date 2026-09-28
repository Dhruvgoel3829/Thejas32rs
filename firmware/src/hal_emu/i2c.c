/*
 * VEGA Pilot Firmware - I2C HAL (Emulator Backend)
 * Used for Barometer (MS5611), Compass (HMC5883L)
 */

#include "config.h"
#include "types.h"
#include "hal/baro.h"
#include "hal/mag.h"
#include "hal/time.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

static inline void mmio_write8(uint32_t addr, uint8_t val) {
    *(volatile uint8_t *)addr = val;
}

static inline uint8_t mmio_read8(uint32_t addr) {
    return *(volatile uint8_t *)addr;
}

bool hal_baro_init(void) {
    /* MS5611 at 0x77 - reset command */
    mmio_write32(I2C_ADDR, 0x77);
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x1E);  /* RESET command */
    hal_delay_ms(3);
    return true;
}

bool hal_baro_read(baro_raw_t *out) {
    /* MS5611: Start D1 (pressure) conversion, then read */
    mmio_write32(I2C_ADDR, 0x77);
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x40);  /* D1 conversion, OSR=256 */
    hal_delay_ms(1);

    /* Read ADC result */
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x00);  /* Read ADC */
    hal_delay_ms(1);

    mmio_write32(I2C_CR, I2C_CR_START | I2C_CR_READ);
    uint8_t msb = mmio_read8(I2C_DATA);
    uint8_t mid = mmio_read8(I2C_DATA);
    uint8_t lsb = mmio_read8(I2C_DATA);

    uint32_t raw_pressure = ((uint32_t)msb << 16) | ((uint32_t)mid << 8) | lsb;

    /* TODO: Read calibration coefficients and compute compensated pressure */
    out->pressure_pa = raw_pressure * 0.1f;  /* placeholder */
    out->temp_c = 25.0f;
    out->t_us = hal_time_us();

    return true;
}

bool hal_mag_init(void) {
    /* HMC5883L at 0x1E */
    mmio_write32(I2C_ADDR, 0x1E);

    /* Config A: 8 samples, 15 Hz, normal mode */
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x00);  /* Config A register */
    mmio_write32(I2C_DATA, 0x70);

    /* Config B: Gain = 5 (0xA0) */
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x01);  /* Config B register */
    mmio_write32(I2C_DATA, 0xA0);

    /* Mode: Continuous measurement (0x00) */
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x02);  /* Mode register */
    mmio_write32(I2C_DATA, 0x00);

    return true;
}

bool hal_mag_read(mag_raw_t *out) {
    mmio_write32(I2C_ADDR, 0x1E);

    /* Read data registers 0x03-0x08 */
    mmio_write32(I2C_CR, I2C_CR_START);
    mmio_write32(I2C_DATA, 0x03);  /* Start at X MSB */

    mmio_write32(I2C_CR, I2C_CR_START | I2C_CR_READ);
    int16_t mx = ((int16_t)mmio_read8(I2C_DATA) << 8) | mmio_read8(I2C_DATA);
    int16_t mz = ((int16_t)mmio_read8(I2C_DATA) << 8) | mmio_read8(I2C_DATA);
    int16_t my = ((int16_t)mmio_read8(I2C_DATA) << 8) | mmio_read8(I2C_DATA);

    out->mx = mx;
    out->my = my;
    out->mz = mz;
    out->t_us = hal_time_us();

    return true;
}