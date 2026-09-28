#ifndef SCHED_H
#define SCHED_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*sched_task_fn)(void *arg);

typedef struct {
    sched_task_fn fn;
    uint32_t period_us;
    time_us_t last_run;
    uint32_t max_cycles;
    uint32_t overruns;
    const char *name;
} sched_task_t;

void sched_run(time_us_t now_us, sched_task_t *tasks, uint8_t count);
void sched_report_cycles(const sched_task_t *tasks, uint8_t count);

#ifdef __cplusplus
}
#endif

#endif /* SCHED_H */