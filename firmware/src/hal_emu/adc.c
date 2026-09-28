/*
 * VEGA Pilot Firmware - ADC HAL (Emulator Backend)
 * Battery voltage, current, RSSI
 */

#include "config.h"
#include "types.h"
#include "hal/adc.h"
#include "hal/time.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_adc_init(void) {
    /* Enable ADC, continuous mode, channel 0 */
    mmio_write32(ADC_CTRL, ADC_CTRL_EN | ADC_CTRL_CONT | (0 << 4));
    return true;
}

bool hal_adc_read(adc_raw_t *out) {
    /* Read channel 0: Battery voltage */
    mmio_write32(ADC_CTRL, ADC_CTRL_EN | ADC_CTRL_CONT | (0 << 4));
    hal_delay_us(10);
    uint32_t raw_v = mmio_read32(ADC_DATA);

    /* Read channel 1: Current */
    mmio_write32(ADC_CTRL, ADC_CTRL_EN | ADC_CTRL_CONT | (1 << 4));
    hal_delay_us(10);
    uint32_t raw_i = mmio_read32(ADC_DATA);

    /* Read channel 2: RSSI */
    mmio_write32(ADC_CTRL, ADC_CTRL_EN | ADC_CTRL_CONT | (2 << 4));
    hal_delay_us(10);
    uint32_t raw_rssi = mmio_read32(ADC_DATA);

    /* Convert to physical units (placeholder calibration) */
    /* Emulator synthetic: ch0=4500 (12-bit), 3.3V ref -> 3.62V */
    out->batt_mv = (uint16_t)((raw_v * 3300) / 4095);
    out->batt_ma = (uint16_t)((raw_i - 2048) * 1000 / 2048);  /* +/- range */
    out->rssi = (uint16_t)raw_rssi;

    return true;
}