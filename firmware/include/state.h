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

/* Interface version - checked at link time via static_assert in modules */
#define STATE_IFACE_VERSION 0x00010000  /* Major.Minor.Patch = 1.0.0 */

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

/* Failsafe status bitmask */
#define FAILSAFE_NONE          0x00
#define FAILSAFE_LOW_BATT      0x01
#define FAILSAFE_CRIT_BATT     0x02
#define FAILSAFE_RC_LOSS       0x04
#define FAILSAFE_GCS_LOSS      0x08
#define FAILSAFE_GNSS_LOSS     0x10
#define FAILSAFE_ATT_FAULT     0x20

/* Vehicle state - filled by estimators (human's code) or stubs */
/* SINGLE WRITER PER FIELD - see ARCHITECTURE.md for ownership table */
typedef struct {
    /* Estimated attitude (writer: attitude estimator) */
    float roll;       /* rad */
    float pitch;      /* rad */
    float yaw;        /* rad */

    /* Estimated angular rates (writer: IMU task) */
    float rate_x;     /* rad/s */
    float rate_y;     /* rad/s */
    float rate_z;     /* rad/s */

    /* Estimated altitude and climb (writer: altitude estimator) */
    float alt_m;      /* m, relative to home/launch */
    float climb_mps;  /* m/s */

    /* Estimated position (writer: position estimator) */
    int32_t lat_e7;   /* deg * 1e7 */
    int32_t lon_e7;   /* deg * 1e7 */
    float vn;         /* m/s, north */
    float ve;         /* m/s, east */

    /* Validity flags (writers: respective estimators) */
    bool att_valid;
    bool alt_valid;
    bool pos_valid;

    /* Health / inputs */
    uint16_t batt_mv;         /* writer: ADC task */
    uint16_t batt_ma;         /* writer: ADC task */
    uint8_t batt_pct;         /* writer: ADC task */

    uint8_t gnss_fix;         /* writer: GNSS UART task */
    uint8_t gnss_sats;        /* writer: GNSS UART task */
    time_ms_t gnss_last_ok_ms; /* writer: GNSS UART task */

    bool rc_link_ok;          /* writer: RC UART task */
    time_ms_t rc_last_ms;     /* writer: RC UART task */

    time_ms_t gcs_last_heartbeat_ms; /* writer: MAVLink RX task */

    bool armed;               /* writer: mode manager (arming logic) */

    /* New fields for M1+ */
    bool landed;              /* writer: land detector (human) */
    ctrl_request_t current_mode; /* writer: mode manager */
    uint8_t failsafe_status;  /* writer: failsafe manager (bitmask) */
    time_us_t timestamp;      /* writer: scheduler (per cycle) */

    home_t home;              /* set on arm (writer: mode manager) */
} vehicle_state_t;

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