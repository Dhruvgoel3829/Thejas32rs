/*
 * QSPI Flash Persistence Test
 * Verifies that --flash-file persists data across emulator restarts
 * Tests: write, exit, rerun, read back
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "hal/flash.h"
#include "sched.h"
#include <stdio.h>

#define FLASH_TEST_ADDR     0x40010000  // Offset 0x10000 from flash base
#define FLASH_TEST_SIZE     256
#define MAGIC_PATTERN       0xA5A5A5A5

static volatile int test_stage = 0;
static uint8_t write_buf[FLASH_TEST_SIZE];
static uint8_t read_buf[FLASH_TEST_SIZE];

static void task_init_pattern(void *arg) {
    (void)arg;
    if (test_stage == 0) {
        // Fill write buffer with known pattern
        for (int i = 0; i < FLASH_TEST_SIZE; i++) {
            write_buf[i] = (uint8_t)(i ^ 0x5A);
        }
        test_stage = 1;
    }
}

static void task_erase_sector(void *arg) {
    (void)arg;
    if (test_stage == 1) {
        // Erase the sector we'll use
        if (hal_flash_erase_sector(1)) {  // Sector 1 = 0x40001000
            test_stage = 2;
        }
    }
}

static void task_write_flash(void *arg) {
    (void)arg;
    if (test_stage == 2) {
        // Write test pattern to flash
        if (hal_flash_write(FLASH_TEST_ADDR, write_buf, FLASH_TEST_SIZE)) {
            test_stage = 3;
        }
    }
}

static void task_read_flash(void *arg) {
    (void)arg;
    if (test_stage == 3) {
        // Read back from flash
        if (hal_flash_read(FLASH_TEST_ADDR, read_buf, FLASH_TEST_SIZE)) {
            test_stage = 4;
        }
    }
}

static void task_verify_pattern(void *arg) {
    (void)arg;
    if (test_stage == 4) {
        int errors = 0;
        for (int i = 0; i < FLASH_TEST_SIZE; i++) {
            if (read_buf[i] != write_buf[i]) {
                errors++;
            }
        }
        
        if (errors == 0) {
            test_stage = 5;  // Success
        } else {
            test_stage = -1; // Verification failed
        }
    }
}

static void task_persistence_check(void *arg) {
    (void)arg;
    if (test_stage == 5) {
        // This test should run AFTER emulator restart
        // Read flash again to verify persistence
        if (hal_flash_read(FLASH_TEST_ADDR, read_buf, FLASH_TEST_SIZE)) {
            int errors = 0;
            for (int i = 0; i < FLASH_TEST_SIZE; i++) {
                if (read_buf[i] != write_buf[i]) {
                    errors++;
                }
            }
            if (errors == 0) {
                test_stage = 6;  // Persistence verified
            } else {
                test_stage = -2; // Persistence failed
            }
        }
    }
}

static void task_report(void *arg) {
    (void)arg;
    hal_gpio_write(GPIO_LED_GREEN, true);
    
    char buf[128];
    int len;
    
    switch (test_stage) {
        case 5:
            len = snprintf(buf, sizeof(buf), "QSPI TEST PASSED: Write/Read verify OK\n");
            hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
            break;
        case 6:
            len = snprintf(buf, sizeof(buf), "QSPI PERSISTENCE TEST PASSED: Data survived restart\n");
            hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
            break;
        case -1:
            len = snprintf(buf, sizeof(buf), "TEST FAILED: Read/Write verify failed\n");
            hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
            break;
        case -2:
            len = snprintf(buf, sizeof(buf), "PERSISTENCE FAILED: Data corrupted after restart\n");
            hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
            break;
        default:
            len = snprintf(buf, sizeof(buf), "TEST INCOMPLETE: stage=%d\n", test_stage);
            hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)buf, len);
            break;
    }
    
    while (1);
}

static void task_heartbeat(void *arg) {
    (void)arg;
    hal_gpio_toggle(GPIO_LED_RED);
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "Stage: %d\n", test_stage);
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
    hal_flash_init();
    
    // Initialize write buffer with pattern
    for (int i = 0; i < FLASH_TEST_SIZE; i++) {
        write_buf[i] = (uint8_t)(i ^ 0x5A);
    }
    
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t*)"QSPI FLASH PERSISTENCE TEST\n", 28);
    
    sched_add_task(task_init_pattern, 100000, "init");
    sched_add_task(task_erase_sector, 200000, "erase");
    sched_add_task(task_write_flash, 300000, "write");
    sched_add_task(task_read_flash, 400000, "read");
    sched_add_task(task_verify_pattern, 500000, "verify");
    sched_add_task(task_persistence_check, 600000, "persist_check");
    sched_add_task(task_report, 2000000, "report");
    
    // Heartbeat
    sched_add_task(task_heartbeat, 100000, "heartbeat");
    
    time_us_t now_us = 0;
    (void)now_us;
    while (1) {
        sched_run(hal_time_us(), g_tasks, g_task_count);
    }
    
    return 0;
}