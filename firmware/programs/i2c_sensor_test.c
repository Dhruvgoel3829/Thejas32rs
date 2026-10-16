/*
 * I2C Sensor Test - HMC5883L Compass
 * Tests HMC5883L read-6 burst
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "hal/i2c.h"
#include "sched.h"
#include <stdio.h>

#define HMC5883L_ADDR     0x1E

// HMC5883L registers
#define HMC5883L_DATA_X_MSB  0x03
#define HMC5883L_DATA_X_LSB  0x04
#define HMC5883L_DATA_Z_MSB  0x05
#define HMC5883L_DATA_Z_LSB  0x06
#define HMC5883L_DATA_Y_MSB  0x07
#define HMC5883L_DATA_Y_LSB  0x08
#define HMC5883L_STATUS      0x09
#define HMC5883L_CONFIG_A    0x00
#define HMC5883L_CONFIG_B    0x01
#define HMC5883L_MODE        0x02

static void task_hmc5883l_read(void *arg) {
    (void)arg;
    // Read 6 bytes starting from X MSB (0x03)
    hal_i2c_write(HMC5883L_ADDR, HMC5883L_DATA_X_MSB, 6, NULL);
}

static void task_heartbeat(void *arg) {
    (void)arg;
    hal_gpio_toggle(GPIO_LED_RED);
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "Running...\n");
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
}

static sched_task_t g_tasks[8];
static uint8_t g_task_count = 0;

static void sched_add_task(sched_task_fn fn, uint32_t period_us, const char *name) {
    if (g_task_count >= 8) return;
    g_tasks[g_task_count].fn = fn;
    g_tasks[g_task_count].period_us = period_us;
    g_tasks[g_task_count].last_run = 0;
    g_tasks[g_task_count].max_cycles = 0;
    g_tasks[g_task_count].min_cycles = UINT32_MAX;
    g_tasks[g_task_count].total_cycles = 0;
    g_tasks[g_task_count].run_count = 0;
    g_tasks[g_task_count].overruns = 0;
    g_tasks[g_task_count].name = name;
    g_task_count++;
}

int main(void) {
    hal_time_init();
    hal_uart_init(UART_ID_TELEM, 115200);
    hal_uart_init(UART_ID_GNSS, 9600);
    hal_uart_init(UART_ID_RC, 100000);
    hal_gpio_init();
    hal_i2c_init();
    
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"I2C SENSOR TEST: HMC5883L\n", 28);
    
    // Initialize HMC5883L
    // Config A: 8 samples, 15Hz, normal measurement
    hal_i2c_write(HMC5883L_ADDR, HMC5883L_CONFIG_A, 1, (uint8_t[]){0x70});
    // Config B: Gain = 5 (0xA0)
    hal_i2c_write(HMC5883L_ADDR, HMC5883L_CONFIG_B, 1, (uint8_t[]){0xA0});
    // Mode: Continuous measurement
    hal_i2c_write(HMC5883L_ADDR, HMC5883L_MODE, 1, (uint8_t[]){0x00});
    
    // Schedule tasks
    sched_add_task(task_hmc5883l_read, 100000, "hmc_read");
    sched_add_task(task_heartbeat, 100000, "heartbeat");
    
    while (1) {
        sched_run(hal_time_us(), g_tasks, g_task_count);
    }
    
    return 0;
}