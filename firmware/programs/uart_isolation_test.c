/*
 * UART Isolation Test
 * Verifies that UART1 (GNSS) and UART2 (RC) are isolated from UART0 (Telemetry)
 * Each UART should only receive data on its own socket
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "sched.h"
#include <stdio.h>

#define TEST_STRING_UART0 "UART0_TELEMETRY_TEST"
#define TEST_STRING_UART1 "UART1_GNSS_TEST"
#define TEST_STRING_UART2 "UART2_RC_TEST"

static volatile int uart0_rx_count = 0;
static volatile int uart1_rx_count = 0;
static volatile int uart2_rx_count = 0;
static volatile int uart0_cross_rx = 0;
static volatile int uart1_cross_rx = 0;
static volatile int uart2_cross_rx = 0;

static uint8_t uart0_rx_buf[64];
static uint8_t uart1_rx_buf[64];
static uint8_t uart2_rx_buf[64];

static void task_uart0_tx(void *arg) {
    (void)arg;
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)TEST_STRING_UART0, sizeof(TEST_STRING_UART0) - 1);
}

static void task_uart1_tx(void *arg) {
    (void)arg;
    hal_uart_write_nb(UART_ID_GNSS, (uint8_t*)TEST_STRING_UART1, sizeof(TEST_STRING_UART1) - 1);
}

static void task_uart2_tx(void *arg) {
    (void)arg;
    hal_uart_write_nb(UART_ID_RC, (uint8_t*)TEST_STRING_UART2, sizeof(TEST_STRING_UART2) - 1);
}

static void task_uart0_rx(void *arg) {
    (void)arg;
    uint16_t len;
    if (hal_uart_read(UART_ID_TELEM, uart0_rx_buf, sizeof(uart0_rx_buf), &len)) {
        uart0_rx_count += len;
    }
    // Check for cross-contamination
    if (hal_uart_read(UART_ID_GNSS, uart0_rx_buf, sizeof(uart0_rx_buf), &len)) {
        uart0_cross_rx += len;
    }
    if (hal_uart_read(UART_ID_RC, uart0_rx_buf, sizeof(uart0_rx_buf), &len)) {
        uart0_cross_rx += len;
    }
}

static void task_uart1_rx(void *arg) {
    (void)arg;
    uint16_t len;
    if (hal_uart_read(UART_ID_GNSS, uart1_rx_buf, sizeof(uart1_rx_buf), &len)) {
        uart1_rx_count += len;
    }
    if (hal_uart_read(UART_ID_TELEM, uart1_rx_buf, sizeof(uart1_rx_buf), &len)) {
        uart1_cross_rx += len;
    }
    if (hal_uart_read(UART_ID_RC, uart1_rx_buf, sizeof(uart1_rx_buf), &len)) {
        uart1_cross_rx += len;
    }
}

static void task_uart2_rx(void *arg) {
    (void)arg;
    uint16_t len;
    if (hal_uart_read(UART_ID_RC, uart2_rx_buf, sizeof(uart2_rx_buf), &len)) {
        uart2_rx_count += len;
    }
    if (hal_uart_read(UART_ID_TELEM, uart2_rx_buf, sizeof(uart2_rx_buf), &len)) {
        uart2_cross_rx += len;
    }
    if (hal_uart_read(UART_ID_GNSS, uart2_rx_buf, sizeof(uart2_rx_buf), &len)) {
        uart2_cross_rx += len;
    }
}

static void task_report(void *arg) {
    (void)arg;
    hal_gpio_write(GPIO_LED_GREEN, true);
    
    // Report results
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"UART Isolation Test Results:\n", 29);
    
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "UART0 RX: %d bytes, Cross: %d\n", uart0_rx_count, uart0_cross_rx);
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
    
    len = snprintf(buf, sizeof(buf), "UART1 RX: %d bytes, Cross: %d\n", uart1_rx_count, uart1_cross_rx);
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
    
    len = snprintf(buf, sizeof(buf), "UART2 RX: %d bytes, Cross: %d\n", uart2_rx_count, uart2_cross_rx);
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
    
    // Test passes if no cross-contamination
    if (uart0_cross_rx == 0 && uart1_cross_rx == 0 && uart2_cross_rx == 0) {
        hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"TEST PASSED: No cross-contamination\n", 36);
    } else {
        hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"TEST FAILED: Cross-contamination detected\n", 40);
    }
    
    // Halt
    while (1);
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
    
    // TX tasks - send test strings
    sched_add_task(task_uart0_tx, 100000, "uart0_tx");   // 10 Hz
    sched_add_task(task_uart1_tx, 100000, "uart1_tx");   // 10 Hz
    sched_add_task(task_uart2_tx, 100000, "uart2_tx");   // 10 Hz
    
    // RX tasks - check for data
    sched_add_task(task_uart0_rx, 10000, "uart0_rx");    // 100 Hz
    sched_add_task(task_uart1_rx, 10000, "uart1_rx");    // 100 Hz
    sched_add_task(task_uart2_rx, 10000, "uart2_rx");    // 100 Hz
    
    // Report after 2 seconds
    sched_add_task(task_report, 500000, "report");
    
    time_us_t now_us = 0;
    while (1) {
        now_us = hal_time_us();
        sched_run(now_us, g_tasks, g_task_count);
    }
    
    return 0;
}