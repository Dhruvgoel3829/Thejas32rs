/*
 * VEGA Pilot Firmware - Main Entry Point
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include "hal/imu.h"
#include "hal/baro.h"
#include "hal/mag.h"
#include "hal/gnss.h"
#include "hal/adc.h"
#include "hal/rc.h"
#include "hal/pwm.h"
#include "sched.h"
#include "state.h"
#include "ctrl_stub.h"
#include <stdio.h>

/* Task function prototypes */
static void task_control(void *arg);
static void task_imu(void *arg);
static void task_baro(void *arg);
static void task_mag(void *arg);
static void task_gnss_uart(void *arg);
static void task_rc_uart(void *arg);
static void task_adc(void *arg);
static void task_mode_failsafe(void *arg);
static void task_mavlink_rx(void *arg);
static void task_mavlink_tx(void *arg);
static void task_logger(void *arg);
static uint32_t stack_high_water_mark(void);

/* Static task table */
static sched_task_t g_tasks[SCHED_MAX_TASKS];
static uint8_t g_task_count = 0;

/* Baro state machine state */
typedef enum {
    BARO_IDLE,
    BARO_CONVERT_D1,
    BARO_WAIT_D1,
    BARO_READ_D1,
    BARO_CONVERT_D2,
    BARO_WAIT_D2,
    BARO_READ_D2
} baro_state_t;

static baro_state_t g_baro_state = BARO_IDLE;
static time_us_t g_baro_start_us = 0;

static void task_control(void *arg) {
    (void)arg;
    if (g_ctrl_interface && g_ctrl_interface->update) {
        g_ctrl_interface->update(&g_vehicle_state, &g_setpoints, &g_motor_out);
    }
    /* Apply motor outputs via HAL */
    for (uint8_t i = 0; i < g_motor_out.count; i++) {
        hal_pwm_set_norm(i, g_motor_out.motor[i]);
    }
}

static void task_imu(void *arg) {
    (void)arg;
    imu_raw_t imu;
    if (hal_imu_read(&imu)) {
        /* Raw gyro LSB to rad/s: 16.4 LSB/(deg/s) -> 1/16.4 * pi/180 = 0.001064 rad/s per LSB */
        g_vehicle_state.rate_x = imu.gx * 0.001064f;
        g_vehicle_state.rate_y = imu.gy * 0.001064f;
        g_vehicle_state.rate_z = imu.gz * 0.001064f;
        g_vehicle_state.att_valid = true;
    }
}

static void task_baro(void *arg) {
    (void)arg;
    time_us_t now = hal_time_us();
    baro_raw_t baro;

    switch (g_baro_state) {
        case BARO_IDLE:
            if (hal_baro_read(&baro)) {
                /* Start D1 (pressure) conversion */
                g_baro_state = BARO_CONVERT_D1;
            }
            break;
        case BARO_CONVERT_D1:
            /* Non-blocking: conversion takes ~9ms at OSR=4096 */
            g_baro_state = BARO_WAIT_D1;
            g_baro_start_us = now;
            break;
        case BARO_WAIT_D1:
            if (now - g_baro_start_us >= 10000) {  /* 10ms wait */
                g_baro_state = BARO_READ_D1;
            }
            break;
        case BARO_READ_D1:
            if (hal_baro_read(&baro)) {
                g_vehicle_state.alt_m = baro.pressure_pa * 0.0001f;  /* placeholder */
                g_vehicle_state.alt_valid = true;
                /* Start D2 (temperature) conversion */
                g_baro_state = BARO_CONVERT_D2;
            }
            break;
        case BARO_CONVERT_D2:
            g_baro_state = BARO_WAIT_D2;
            g_baro_start_us = now;
            break;
        case BARO_WAIT_D2:
            if (now - g_baro_start_us >= 10000) {
                g_baro_state = BARO_READ_D2;
            }
            break;
        case BARO_READ_D2:
            if (hal_baro_read(&baro)) {
                /* Temperature compensation would go here */
                g_baro_state = BARO_IDLE;
            }
            break;
    }
}

static void task_mag(void *arg) {
    (void)arg;
    mag_raw_t mag;
    if (hal_mag_read(&mag)) {
        /* TODO: Compensate and update heading */
        (void)mag;
    }
}

static void task_gnss_uart(void *arg) {
    (void)arg;
    /* Non-blocking UART drain -> NMEA parser */
    uint8_t buf[64];
    uint16_t len;
    if (hal_uart_read(UART_ID_GNSS, buf, sizeof(buf), &len)) {
        /* TODO: Feed to NMEA parser, update gnss_fix, lat/lon, sats */
        (void)buf;
        (void)len;
    }
}

static void task_rc_uart(void *arg) {
    (void)arg;
    /* Non-blocking UART drain -> SBUS/CRSF parser */
    uint8_t buf[64];
    uint16_t len;
    if (hal_uart_read(UART_ID_RC, buf, sizeof(buf), &len)) {
        /* TODO: Feed to RC parser, update rc_link_ok, channels */
        (void)buf;
        (void)len;
    }
}

static void task_adc(void *arg) {
    (void)arg;
    adc_raw_t adc;
    if (hal_adc_read(&adc)) {
        g_vehicle_state.batt_mv = adc.batt_mv;
        g_vehicle_state.batt_ma = adc.batt_ma;
        /* Simple battery percentage: 3.0V=0%, 4.2V=100% per cell * 4S */
        if (adc.batt_mv >= 16800) g_vehicle_state.batt_pct = 100;
        else if (adc.batt_mv <= 12000) g_vehicle_state.batt_pct = 0;
        else g_vehicle_state.batt_pct = (adc.batt_mv - 12000) * 100 / 4800;
    }
}

static void task_mode_failsafe(void *arg) {
    (void)arg;
    /* TODO: Call mode manager and failsafe manager */
    g_vehicle_state.timestamp = hal_time_us();
}

static void task_mavlink_rx(void *arg) {
    (void)arg;
    /* TODO: Drain UART RX ring and parse MAVLink */
}

static void task_mavlink_tx(void *arg) {
    (void)arg;
    /* TODO: Send periodic telemetry */
}

static void task_logger(void *arg) {
    (void)arg;
    /* TODO: Flush log buffer to flash */
    /* Stack high-water mark: full 224 KB scan costs ~285k retired, so run
     * it at ~0.1 Hz (every 100th logger run) and report the cached value. */
    static uint32_t logger_runs = 0;
    if ((logger_runs % 100) == 0) {
        g_vehicle_state.stack_hwm_bytes = stack_high_water_mark();
    }
    logger_runs++;

    /* Overrun report for `make emu-test` (fails on "OVERRUNS DETECTED").
     * Prints once at startup (HWM), then only when the overrun count grows. */
    static uint32_t last_overruns = 0;
    static uint32_t report_count = 0;
    uint32_t total_overruns = 0;
    for (uint8_t i = 0; i < g_task_count; i++) {
        total_overruns += g_tasks[i].overruns;
    }
    if (report_count == 0 || total_overruns != last_overruns) {
        char buf[96];
        int len = snprintf(buf, sizeof(buf),
            "STACK HWM: %lu bytes free, TASK OVERRUNS: %lu%s\n",
            (unsigned long)g_vehicle_state.stack_hwm_bytes, (unsigned long)total_overruns,
            total_overruns > 0 ? " OVERRUNS DETECTED" : "");
        hal_uart_write_nb(UART_ID_TELEM, (uint8_t *)buf, (uint16_t)len);
        last_overruns = total_overruns;
    }
    report_count++;
}

/* Stack high-water mark scan
 * Stack grows DOWN from __stack_top. Paint region is from __bss_end (low) to __stack_top (high).
 * Scan from LOW end (__bss_end) upward toward __stack_top.
 * Find the LOWEST address (deepest penetration) that has been overwritten (paint != 0xDEADBEEF).
 * That address gives the historical maximum stack depth. Report bytes from that address to __stack_top.
 */
static uint32_t stack_high_water_mark(void) {
    extern uint32_t __stack_top;
    extern uint32_t __bss_end;
    uint32_t *stack_ptr = (uint32_t *)&__bss_end;
    uint32_t *stack_top = (uint32_t *)&__stack_top;
    uint32_t max_used = 0;  /* historical maximum stack usage in bytes */

    while (stack_ptr < stack_top) {
        if (*stack_ptr != 0xDEADBEEF) {
            uint32_t used = (uint32_t)stack_top - (uint32_t)stack_ptr;
            if (used > max_used) max_used = used;  /* keep DEEPEST (largest used) */
        }
        stack_ptr++;
    }
    return max_used;  /* bytes from deepest overwritten address to __stack_top */
}

static void sched_add_task(sched_task_fn fn, uint32_t period_us, const char *name) {
    if (g_task_count >= SCHED_MAX_TASKS) return;
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
    /* Initialize HAL */
    hal_time_init();
    hal_uart_init(UART_ID_TELEM, 57600);
    hal_uart_init(UART_ID_GNSS, 9600);
    hal_uart_init(UART_ID_RC, 100000);
    hal_gpio_init();
    hal_pwm_init();
    hal_imu_init();
    hal_baro_init();
    hal_mag_init();
    hal_gnss_init();
    hal_adc_init();
    hal_rc_init();

    /* Initialize controller stub */
    if (g_ctrl_interface && g_ctrl_interface->init) {
        g_ctrl_interface->init();
    }

    /* Stack painting for high-water mark */
    extern uint32_t __stack_top;
    extern uint32_t __bss_end;
    uint32_t *stack_ptr = (uint32_t *)&__bss_end;
    uint32_t *stack_top = (uint32_t *)&__stack_top;
    while (stack_ptr < stack_top) {
        *stack_ptr++ = 0xDEADBEEF;
    }

    /* Register tasks */
    sched_add_task(task_control, SCHED_CTRL_PERIOD_US, "control");
    sched_add_task(task_imu, SCHED_IMU_PERIOD_US, "imu");
    sched_add_task(task_baro, SCHED_BARO_PERIOD_US, "baro");
    sched_add_task(task_mag, SCHED_MAG_PERIOD_US, "mag");
    sched_add_task(task_gnss_uart, SCHED_GNSS_UART_PERIOD_US, "gnss_uart");
    sched_add_task(task_rc_uart, SCHED_RC_UART_PERIOD_US, "rc_uart");
    sched_add_task(task_adc, SCHED_ADC_PERIOD_US, "adc");
    sched_add_task(task_mode_failsafe, SCHED_MODE_PERIOD_US, "mode_fs");
    sched_add_task(task_mavlink_rx, SCHED_MAVLINK_RX_PERIOD_US, "mav_rx");
    sched_add_task(task_mavlink_tx, SCHED_MAVLINK_TX_PERIOD_US, "mav_tx");
    sched_add_task(task_logger, SCHED_LOGGER_PERIOD_US, "logger");

    /* Main scheduler loop */
    time_us_t now_us = 0;
    while (1) {
        now_us = hal_time_us();
        sched_run(now_us, g_tasks, g_task_count);
    }

    return 0;
}