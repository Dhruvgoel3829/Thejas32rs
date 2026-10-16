/*
 * VEGA Pilot Firmware - Host Unit Tests
 * Compiles and runs on x86-64 for CI
 */

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <assert.h>

/* Mock HAL for host tests */
#include "config.h"
#include "types.h"
#include "state.h"
#include "sched.h"
#include "hal/time.h"
#include "hal/time_host.h"

/* Test: Scheduler basic functionality */
static void test_sched_basic(void) {
    sched_task_t tasks[2] = {0};
    uint8_t count = 0;
    int call_count_a = 0, call_count_b = 0;

    void task_a(void *arg) { (void)arg; call_count_a++; }
    void task_b(void *arg) { (void)arg; call_count_b++; }

    tasks[count++] = (sched_task_t){.fn = task_a, .period_us = 1000, .last_run = 0};
    tasks[count++] = (sched_task_t){.fn = task_b, .period_us = 2000, .last_run = 0};

    /* At t=0, no task should run (elapsed=0 < period) */
    hal_time_mock_advance_us(0);
    sched_run(hal_time_us(), tasks, count);
    assert(call_count_a == 0);
    assert(call_count_b == 0);

    /* At t=500, still no task should run */
    hal_time_mock_advance_us(500);
    sched_run(hal_time_us(), tasks, count);
    assert(call_count_a == 0);
    assert(call_count_b == 0);

    /* At t=1000, task_a should run (elapsed=1000 >= 1000) */
    hal_time_mock_advance_us(500);
    sched_run(hal_time_us(), tasks, count);
    assert(call_count_a == 1);
    assert(call_count_b == 0);

    /* At t=1500, no task should run (task_a elapsed=500, task_b elapsed=1500) */
    hal_time_mock_advance_us(500);
    sched_run(hal_time_us(), tasks, count);
    assert(call_count_a == 1);
    assert(call_count_b == 0);

    /* At t=2000, both task_a and task_b should run */
    hal_time_mock_advance_us(500);
    sched_run(hal_time_us(), tasks, count);
    assert(call_count_a == 2);
    assert(call_count_b == 1);

    printf("test_sched_basic: PASS\n");
}

/* Test: Vehicle state initialization */
static void test_state_init(void) {
    state_init();
    assert(g_vehicle_state.armed == false);
    assert(g_vehicle_state.att_valid == false);
    assert(g_vehicle_state.alt_valid == false);
    assert(g_vehicle_state.pos_valid == false);
    printf("test_state_init: PASS\n");
}

/* Test: Control stub interface */
static void test_ctrl_stub(void) {
    motor_out_t out = {0};
    setpoints_t sp = {.throttle = 0.5f};

    if (g_ctrl_interface && g_ctrl_interface->init) {
        g_ctrl_interface->init();
    }
    if (g_ctrl_interface && g_ctrl_interface->update) {
        g_ctrl_interface->update(&g_vehicle_state, &sp, &out);
    }

    assert(out.count == 4);
    for (int i = 0; i < 4; i++) {
        assert(out.motor[i] == 500);  /* 0.5 * 1000 */
    }
    printf("test_ctrl_stub: PASS\n");
}

/* Test: Setpoints structure */
static void test_setpoints(void) {
    setpoints_t sp = {0};
    sp.request = CTRL_STABILIZE;
    sp.roll_t = 0.1f;
    sp.pitch_t = -0.1f;
    sp.yaw_rate_t = 0.0f;
    sp.throttle = 0.5f;

    assert(sp.request == CTRL_STABILIZE);
    assert(sp.throttle == 0.5f);
    printf("test_setpoints: PASS\n");
}

/* Test: Parameter bounds */
static void test_param_bounds(void) {
    param_entry_t param = {
        .name = "TEST_PARAM",
        .value = 5.0f,
        .min = 0.0f,
        .max = 10.0f,
        .def = 5.0f
    };

    assert(param.value >= param.min);
    assert(param.value <= param.max);
    printf("test_param_bounds: PASS\n");
}

/* Test: snprintf edge cases (freestanding libc) */
/* NOTE: -Wformat is disabled here: %s/NULL, %f-without-arg and intentional
 * truncation are exactly what this test exercises against OUR snprintf. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wformat"
#pragma GCC diagnostic ignored "-Wformat-truncation"
static void test_snprintf_null(void) {
    char buf[64];
    int n;

    /* %s with NULL prints "(null)", never dereferences */
    n = snprintf(buf, sizeof(buf), "a%sb", (const char *)NULL);
    assert(strcmp(buf, "a(null)b") == 0);
    assert(n == 8);

    /* %d %u %x %c %lu conversions */
    n = snprintf(buf, sizeof(buf), "%d %u %x %c %lu", -5, 7u, 0xAB, 'Q', 123ul);
    assert(strcmp(buf, "-5 7 ab Q 123") == 0);
    (void)n;

    /* truncation stays NUL-terminated within size */
    n = snprintf(buf, 4, "abcdef");
    assert(strcmp(buf, "abc") == 0);

    /* float formats unsupported: printed literally, never crash */
    n = snprintf(buf, sizeof(buf), "%f");
    assert(strcmp(buf, "%f") == 0);
    (void)n;

    printf("test_snprintf_null: PASS\n");
}
#pragma GCC diagnostic pop

/* Main test runner */
int main(void) {
    hal_time_init();
    printf("=== VEGA Pilot Firmware Host Tests ===\n");

    test_sched_basic();
    test_state_init();
    test_ctrl_stub();
    test_setpoints();
    test_param_bounds();
    test_snprintf_null();

    printf("=== All Tests Passed ===\n");
    return 0;
}