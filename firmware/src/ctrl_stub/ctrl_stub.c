/*
 * VEGA Pilot Firmware - Control Stub (PLACEHOLDER)
 * 
 * THIS IS A PLACEHOLDER IMPLEMENTATION.
 * The human will replace this with the actual flight controller.
 * 
 * Do not extend this. It exists only to keep the emulator-based demos running.
 */

#include "config.h"
#include "state.h"
#include "hal/pwm.h"

static void ctrl_stub_init(void) {
    /* Initialize PWM outputs to idle */
    hal_pwm_arm_outputs(false);
    for (int i = 0; i < 4; i++) {
        hal_pwm_set_norm(i, 0);
    }
}

static void ctrl_stub_update(const vehicle_state_t *state, const setpoints_t *sp, motor_out_t *out) {
    (void)state;
    (void)sp;

    /* Placeholder: simple pass-through of throttle to all motors */
    out->count = 4;
    for (int i = 0; i < 4; i++) {
        out->motor[i] = (uint16_t)(sp->throttle * 1000.0f);
    }
}

static void ctrl_stub_reset(void) {
    for (int i = 0; i < 4; i++) {
        hal_pwm_set_norm(i, 0);
    }
}

const ctrl_interface_t ctrl_stub_interface = {
    .init = ctrl_stub_init,
    .update = ctrl_stub_update,
    .reset = ctrl_stub_reset
};