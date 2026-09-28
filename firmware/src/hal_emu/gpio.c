/*
 * VEGA Pilot Firmware - GPIO HAL (Emulator Backend)
 */

#include "config.h"
#include "types.h"
#include "hal/gpio.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

bool hal_gpio_init(void) {
    /* Set all pins as outputs by default */
    mmio_write32(GPIO_DIR, 0xFFFFFFFF);
    mmio_write32(GPIO_DATA_OUT, 0);
    return true;
}

void hal_gpio_write(gpio_pin_t pin, bool val) {
    uint32_t mask = 1u << pin;
    if (val) {
        mmio_write32(GPIO_SET, mask);
    } else {
        mmio_write32(GPIO_CLEAR, mask);
    }
}

bool hal_gpio_read(gpio_pin_t pin) {
    uint32_t data = mmio_read32(GPIO_DATA_OUT);
    return (data >> pin) & 1;
}

void hal_gpio_toggle(gpio_pin_t pin) {
    uint32_t data = mmio_read32(GPIO_DATA_OUT);
    mmio_write32(GPIO_DATA_OUT, data ^ (1u << pin));
}