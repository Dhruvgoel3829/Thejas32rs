#ifndef STATE_H
#define STATE_H

#include "types.h"
#include "hal/imu.h"
#include "hal/baro.h"
#include "hal/mag.h"
#include "hal/gnss.h"
#include "hal/adc.h"
#include "hal/rc.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Vehicle state - filled by estimators (human's code) or stubs */
typedef struct {
    /* Estimated attitude */
    float roll;       /* rad */
    float pitch;      /* rad */
    float yaw;        /* rad */

    /* Estimated angular rates */
    float rate_x;     /* rad/s */
    float rate_y;     /* rad/s */
    float rate_z;     /* rad/s */

    /* Estimated altitude and climb */
    float alt_m;      /* m, relative to home/launch */
    float climb_mps;  /* m/s */

    /* Estimated position */
    int32_t lat_e7;   /* deg * 1e7 */
    int32_t lon_e7;   /* deg * 1e7 */
    float vn;         /* m/s, north */
    float ve;         /* m/s, east */

    /* Validity flags */
    bool att_valid;
    bool alt_valid;
    bool pos_valid;

    /* Health / inputs */
    uint16_t batt_mv;
    uint16_t batt_ma;
    uint8_t batt_pct;

    uint8_t gnss_fix;
    uint8_t gnss_sats;
    time_ms_t gnss_last_ok_ms;

    bool rc_link_ok;
    time_ms_t rc_last_ms;

    time_ms_t gcs_last_heartbeat_ms;

    bool armed;

    home_t home;      /* set on arm */
} vehicle_state_t;

/* Control request from mode manager */
typedef enum {
    CTRL_NONE = 0,
    CTRL_STABILIZE,
    CTRL_ALT_HOLD,
    CTRL_POS_HOLD,
    CTRL_WAYPOINT,
    CTRL_LAND,
    CTRL_DISARMED
} ctrl_request_t;

/* Setpoints from mode manager to controller */
typedef struct {
    ctrl_request_t request;

    /* Attitude/throttle targets (STABILIZE / manual assist) */
    float roll_t;
    float pitch_t;
    float yaw_rate_t;
    float throttle;       /* 0.0 - 1.0 */

    /* Navigation targets (ALT_HOLD / POS_HOLD / WAYPOINT / LAND) */
    int32_t tgt_lat_e7;
    int32_t tgt_lon_e7;
    float tgt_alt_m;

    float max_speed_mps;
    float climb_rate_mps;
    float descend_rate_mps;
} setpoints_t;

/* Motor outputs from controller */
typedef struct {
    uint16_t motor[8];    /* PWM duty 0-1000 or us */
    uint8_t count;        /* number of active motors */
} motor_out_t;

/* Controller interface - human implements this */
typedef struct {
    void (*init)(void);
    void (*update)(const vehicle_state_t *state, const setpoints_t *sp, motor_out_t *out);
    void (*reset)(void);
} ctrl_interface_t;

extern const ctrl_interface_t *g_ctrl_interface;

void state_init(void);
void state_update_sensors(const imu_raw_t *imu, const baro_raw_t *baro,
                          const mag_raw_t *mag, const gnss_raw_t *gnss,
                          const adc_raw_t *adc, const rc_raw_t *rc);

extern vehicle_state_t g_vehicle_state;
extern setpoints_t g_setpoints;
extern motor_out_t g_motor_out;

#ifdef __cplusplus
}
#endif

#endif /* STATE_H */