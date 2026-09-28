/*
 * VEGA Pilot Firmware - Host HAL Stubs
 * Mock implementations for host unit tests
 */

#include "config.h"
#include "types.h"
#include "hal/imu.h"
#include "hal/baro.h"
#include "hal/mag.h"
#include "hal/gnss.h"
#include "hal/adc.h"
#include "hal/rc.h"
#include "hal/pwm.h"
#include "hal/gpio.h"
#include "hal/uart.h"

bool hal_imu_init(void) { return true; }
bool hal_imu_read(imu_raw_t *out) { (void)out; return false; }

bool hal_baro_init(void) { return true; }
bool hal_baro_read(baro_raw_t *out) { (void)out; return false; }

bool hal_mag_init(void) { return true; }
bool hal_mag_read(mag_raw_t *out) { (void)out; return false; }

bool hal_gnss_init(void) { return true; }
bool hal_gnss_read(gnss_raw_t *out) { (void)out; return false; }
void nmea_parser_feed(nmea_rx_t *rx, uint8_t byte) { (void)rx; (void)byte; }
bool nmea_parser_parse(nmea_rx_t *rx, gnss_raw_t *out) { (void)rx; (void)out; return false; }

bool hal_adc_init(void) { return true; }
bool hal_adc_read(adc_raw_t *out) { (void)out; return false; }

bool hal_rc_init(void) { return true; }
bool hal_rc_read(rc_raw_t *out) { (void)out; return false; }
void rc_parser_feed(rc_rx_t *rx, uint8_t byte) { (void)rx; (void)byte; }
bool rc_parser_parse(rc_rx_t *rx, rc_raw_t *out) { (void)rx; (void)out; return false; }

bool hal_pwm_init(void) { return true; }
bool hal_pwm_set_us(uint8_t ch, uint16_t us) { (void)ch; (void)us; return true; }
bool hal_pwm_set_norm(uint8_t ch, uint16_t norm_0_1000) { (void)ch; (void)norm_0_1000; return true; }
bool hal_pwm_arm_outputs(bool arm) { (void)arm; return true; }
bool hal_pwm_set_freq(uint32_t freq_hz) { (void)freq_hz; return true; }

bool hal_gpio_init(void) { return true; }
void hal_gpio_write(gpio_pin_t pin, bool val) { (void)pin; (void)val; }
bool hal_gpio_read(gpio_pin_t pin) { (void)pin; return false; }
void hal_gpio_toggle(gpio_pin_t pin) { (void)pin; }

bool hal_uart_init(uart_id_t id, uint32_t baud) { (void)id; (void)baud; return true; }
bool hal_uart_write_nb(uart_id_t id, const uint8_t *buf, uint16_t len) { (void)id; (void)buf; (void)len; return true; }
bool hal_uart_read(uart_id_t id, uint8_t *buf, uint16_t max_len, uint16_t *out_len) { (void)id; (void)buf; (void)max_len; *out_len = 0; return false; }