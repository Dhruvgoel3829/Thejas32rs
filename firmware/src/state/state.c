/*
 * VEGA Pilot Firmware - Vehicle State Module
 * Interfaces between estimators and controller
 */

#include "config.h"
#include "state.h"
#include "hal/imu.h"
#include "hal/baro.h"
#include "hal/mag.h"
#include "hal/gnss.h"
#include "hal/adc.h"
#include "hal/rc.h"

/* Default controller interface (stub) */
extern const ctrl_interface_t ctrl_stub_interface;

const ctrl_interface_t *g_ctrl_interface = &ctrl_stub_interface;

/* Global state instances */
vehicle_state_t g_vehicle_state = {0};
setpoints_t g_setpoints = {0};
motor_out_t g_motor_out = {0};

void state_init(void) {
    /* Initialize vehicle state to safe defaults */
    g_vehicle_state.armed = false;
    g_vehicle_state.att_valid = false;
    g_vehicle_state.alt_valid = false;
    g_vehicle_state.pos_valid = false;
    g_vehicle_state.rc_link_ok = false;
    g_vehicle_state.gnss_fix = 0;
    g_vehicle_state.gnss_sats = 0;
    g_vehicle_state.batt_pct = 100;
}

void state_update_sensors(const imu_raw_t *imu, const baro_raw_t *baro,
                          const mag_raw_t *mag, const gnss_raw_t *gnss,
                          const adc_raw_t *adc, const rc_raw_t *rc) {
    /* Stub: copy raw sensor data to state with minimal processing */
    (void)mag;  /* unused in stub */

    if (imu) {
        g_vehicle_state.rate_x = imu->gx * 0.001f;
        g_vehicle_state.rate_y = imu->gy * 0.001f;
        g_vehicle_state.rate_z = imu->gz * 0.001f;
        g_vehicle_state.att_valid = true;
    }
    if (baro) {
        g_vehicle_state.alt_m = baro->pressure_pa * 0.0001f;  /* placeholder */
        g_vehicle_state.alt_valid = true;
    }
    if (gnss) {
        g_vehicle_state.lat_e7 = gnss->lat_e7;
        g_vehicle_state.lon_e7 = gnss->lon_e7;
        g_vehicle_state.gnss_fix = gnss->fix_type;
        g_vehicle_state.gnss_sats = gnss->sats;
        g_vehicle_state.pos_valid = (gnss->fix_type >= 3);
    }
    if (adc) {
        g_vehicle_state.batt_mv = adc->batt_mv;
        g_vehicle_state.batt_ma = adc->batt_ma;
    }
    if (rc) {
        g_vehicle_state.rc_link_ok = rc->link_ok;
    }
}