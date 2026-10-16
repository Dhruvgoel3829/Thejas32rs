/*
 * Strict Memory Mode Test - NULL Dereference
 * Verifies that --strict-mem causes a fault on NULL dereference
 * instead of returning 0 silently.
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "sched.h"
#include <stdio.h>

static volatile int test_stage = 0;
static volatile int fault_triggered = 0;

static void task_test_null_read(void *arg) {
    (void)arg;
    if (test_stage == 0) {
        test_stage = 1;
        // Try to read from NULL address (0x00000000)
        volatile uint32_t *null_ptr = (volatile uint32_t *)0x00000000;
        uint32_t val = *null_ptr;  // Should fault in --strict-mem mode
        (void)val;
    }
}

static void task_test_null_write(void *arg) {
    (void)arg;
    if (test_stage == 1) {
        test_stage = 2;
        // Try to write to NULL address
        volatile uint32_t *null_ptr = (volatile uint32_t *)0x00000000;
        *null_ptr = 0xDEADBEEF;  // Should fault in --strict-mem mode
    }
}

static void task_test_unmapped_read(void *arg) {
    (void)arg;
    if (test_stage == 2) {
        test_stage = 3;
        // Try to read from unmapped address (0x00010000)
        volatile uint32_t *unmapped_ptr = (volatile uint32_t *)0x00010000;
        uint32_t val = *unmapped_ptr;  // Should fault in --strict-mem mode
        (void)val;
    }
}

static void task_test_unmapped_write(void *arg) {
    (void)arg;
    if (test_stage == 3) {
        test_stage = 4;
        // Try to write to unmapped address
        volatile uint32_t *unmapped_ptr = (volatile uint32_t *)0x00010000;
        *unmapped_ptr = 0xDEADBEEF;  // Should fault in --strict-mem mode
    }
}

static void task_test_pc_zero(void *arg) {
    (void)arg;
    if (test_stage == 4) {
        test_stage = 5;
        // Try to jump to PC=0 by calling function pointer at NULL
        void (*null_func)(void) = (void (*)(void))0x00000000;
        null_func();  // Should fault in --strict-mem mode
    }
}

static void task_report(void *arg) {
    (void)arg;
    hal_gpio_write(GPIO_LED_GREEN, true);
    
    // If we reach here without faulting, test failed in --strict-mem mode
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"STRICT-MEM TEST: NO FAULT DETECTED - TEST FAILED\n", 46);
    
    // Halt
    while (1);
}

static void task_heartbeat(void *arg) {
    (void)arg;
    hal_gpio_toggle(GPIO_LED_RED);
    // Print test stage
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "Test stage: %d\n", test_stage);
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
    hal_gpio_init();
    
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"STRICT-MEM NULL DEREF TEST\n", 29);
    
    // Test sequence - each test runs once
    sched_add_task(task_test_null_read, 100000, "null_read");
    sched_add_task(task_test_null_write, 300000, "null_write");
    sched_add_task(task_test_unmapped_read, 500000, "unmapped_read");
    sched_add_task(task_test_unmapped_write, 700000, "unmapped_write");
    sched_add_task(task_test_pc_zero, 900000, "pc_zero");
    
    // Heartbeat
    sched_add_task(task_heartbeat, 100000, "heartbeat");
    
    // Report after all tests
    sched_add_task(task_report, 2000000, "report");
    
    time_us_t now_us = 0;
    (void)now_us;
    while (1) {
        sched_run(hal_time_us(), g_tasks, g_task_count);
    }
    
    return 0;
}