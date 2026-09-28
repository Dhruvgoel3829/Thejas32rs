#ifndef HAL_GPIO_H
#define HAL_GPIO_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    GPIO_LED_RED = 0,
    GPIO_LED_GREEN = 1,
    GPIO_LED_BLUE = 2,
    GPIO_BUZZER = 3,
    GPIO_SAFETY_SW = 4,
    GPIO_MAX = 5
} gpio_pin_t;

bool hal_gpio_init(void);
void hal_gpio_write(gpio_pin_t pin, bool val);
bool hal_gpio_read(gpio_pin_t pin);
void hal_gpio_toggle(gpio_pin_t pin);

#ifdef __cplusplus
}
#endif

#endif /* HAL_GPIO_H */