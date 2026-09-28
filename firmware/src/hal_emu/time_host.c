/*
 * VEGA Pilot Firmware - Time HAL (Host Test Stub)
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"

static uint64_t g_mock_time_us = 0;
static uint64_t g_mock_cycles = 0;

void hal_time_init(void) {
    g_mock_time_us = 0;
    g_mock_cycles = 0;
}

time_us_t hal_time_us(void) {
    return g_mock_time_us;
}

time_ms_t hal_time_ms(void) {
    return g_mock_time_us / 1000;
}

cycles_t hal_cycles(void) {
    return g_mock_cycles;
}

void hal_delay_us(uint32_t us) {
    g_mock_time_us += us;
    g_mock_cycles += us * 100;  /* assume 100 MHz */
}

void hal_delay_ms(uint32_t ms) {
    hal_delay_us(ms * 1000);
}

/* Test helper functions */
void hal_time_mock_advance_us(uint32_t us) {
    g_mock_time_us += us;
    g_mock_cycles += us * 100;
}

void hal_time_mock_set_cycles(uint64_t cycles) {
    g_mock_cycles = cycles;
}