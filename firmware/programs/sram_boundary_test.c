/*
 * SRAM Boundary Test
 * Writes a byte at 0x80040000 (first byte past 256 KB SRAM).
 * - With --sram-kb 256 --strict-mem: must FAULT (non-zero exit).
 * - With default 512 KB SRAM: write succeeds, prints OK, halts.
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "sched.h"
#include <stdio.h>

#define SRAM_256K_TOP  ((volatile uint8_t *)0x80040000)

static void task_probe(void *arg) {
    (void)arg;
    /* Probe write at the 256 KB boundary */
    *SRAM_256K_TOP = 0xA5;
    uint8_t back = *SRAM_256K_TOP;

    char buf[64];
    int len = snprintf(buf, sizeof(buf), "SRAM boundary write OK, readback=%u\n", back);
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t *)buf, (uint16_t)len);
    hal_gpio_write(GPIO_LED_GREEN, true);
    while (1) {
    }
}

static sched_task_t g_tasks[2];
static uint8_t g_task_count = 0;

static void sched_add_task(sched_task_fn fn, uint32_t period_us, const char *name) {
    if (g_task_count >= 2) return;
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
    hal_gpio_init();

    sched_add_task(task_probe, 100000, "probe");

    while (1) {
        sched_run(hal_time_us(), g_tasks, g_task_count);
    }

    return 0;
}