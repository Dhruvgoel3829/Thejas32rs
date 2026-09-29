/*
 * VEGA Pilot Firmware - Cooperative Fixed-Rate Scheduler
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "sched.h"

void sched_run(time_us_t now_us, sched_task_t *tasks, uint8_t count) {
    for (uint8_t i = 0; i < count; i++) {
        sched_task_t *t = &tasks[i];
        time_us_t elapsed = now_us - t->last_run;

        if (elapsed >= t->period_us) {
            cycles_t start = hal_cycles();
            t->fn(NULL);
            cycles_t end = hal_cycles();
            uint32_t cycles = (uint32_t)(end - start);

            if (cycles > t->max_cycles) {
                t->max_cycles = cycles;
            }
            if (cycles < t->min_cycles) {
                t->min_cycles = cycles;
            }
            t->total_cycles += cycles;
            t->run_count++;

            /* Check for overrun: instructions retired > period_us * (MTIME_HZ / 1_000_000) */
            uint32_t max_instr_per_period = t->period_us * (MTIME_HZ / 1000000);
            if (cycles > max_instr_per_period) {
                t->overruns++;
            }

            t->last_run = now_us;
        }
    }
}

void sched_report_cycles(const sched_task_t *tasks, uint8_t count) {
    /* TODO: Output via UART or debug command */
    (void)tasks;
    (void)count;
}