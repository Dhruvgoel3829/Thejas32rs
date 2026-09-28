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

/* Task function prototypes */
static void task_control(void *arg);
static void task_state_estimate(void *arg);
static void task_mode_failsafe(void *arg);
static void task_mavlink_rx(void *arg);
static void task_mavlink_tx(void *arg);
static void task_logger(void *arg);

/* Static task table */
static sched_task_t g_tasks[SCHED_MAX_TASKS];
static uint8_t g_task_count = 0;

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

static void task_state_estimate(void *arg) {
    (void)arg;
    /* TODO: Call human's estimator stubs here */
    /* For now, just read raw sensors and copy to state */
    imu_raw_t imu;
    if (hal_imu_read(&imu)) {
        g_vehicle_state.rate_x = imu.gx * 0.001f;  /* placeholder scaling */
        g_vehicle_state.rate_y = imu.gy * 0.001f;
        g_vehicle_state.rate_z = imu.gz * 0.001f;
        g_vehicle_state.att_valid = true;
    }
}

static void task_mode_failsafe(void *arg) {
    (void)arg;
    /* TODO: Call mode manager and failsafe manager */
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
}

static void sched_add_task(sched_task_fn fn, uint32_t period_us, const char *name) {
    if (g_task_count >= SCHED_MAX_TASKS) return;
    g_tasks[g_task_count].fn = fn;
    g_tasks[g_task_count].period_us = period_us;
    g_tasks[g_task_count].last_run = 0;
    g_tasks[g_task_count].max_cycles = 0;
    g_tasks[g_task_count].overruns = 0;
    g_tasks[g_task_count].name = name;
    g_task_count++;
}

int main(void) {
    /* Initialize HAL */
    hal_time_init();
    hal_uart_init(UART_ID_TELEM, 57600);
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

    /* Register tasks */
    sched_add_task(task_control, SCHED_CTRL_PERIOD_US, "control");
    sched_add_task(task_state_estimate, SCHED_STATE_PERIOD_US, "state");
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