#ifndef CONFIG_H
#define CONFIG_H

/*
 * VEGA Pilot Firmware Configuration
 * All tunables and limits in one place.
 */

#ifndef SRAM_BUDGET_BYTES
#define SRAM_BUDGET_BYTES       (256 * 1024)  /* 256 KB budget for real hardware */
#endif

#ifndef STACK_RESERVE_BYTES
#define STACK_RESERVE_BYTES     (32 * 1024)   /* 32 KB stack reserve */
#endif

/* Time base */
#define MTIME_HZ                100000000     /* CLINT mtime frequency (emulator = 100 MHz nominal) */
/* NOTE: hal_time_us() divides mtime by (MTIME_HZ/1000000); MTIME_HZ must
 * stay >= 1 MHz on any port or the divisor becomes 0. */

/* Scheduler task rates (Hz) - final values from M1 profiling */
#define SCHED_CTRL_HZ           1000          /* Control loop: 1000 Hz */
#define SCHED_IMU_HZ            1000          /* IMU SPI read: 1000 Hz */
#define SCHED_BARO_HZ           100           /* Baro I2C state machine: 100 Hz */
#define SCHED_MAG_HZ            75            /* Mag I2C read: 75 Hz */
#define SCHED_GNSS_UART_HZ      100           /* GNSS UART drain: 100 Hz */
#define SCHED_RC_UART_HZ        100           /* RC UART drain: 100 Hz */
#define SCHED_ADC_HZ            100           /* ADC read: 100 Hz */
#define SCHED_MODE_HZ           50            /* Mode/failsafe manager: 50 Hz */
#define SCHED_MAVLINK_RX_HZ     100           /* MAVLink RX parse: 100 Hz */
#define SCHED_MAVLINK_TX_HZ     10            /* MAVLink telemetry TX: 10 Hz base */
#define SCHED_LOGGER_HZ         10            /* Logger flush: 10 Hz */

/* Derived periods in microseconds */
#define SCHED_CTRL_PERIOD_US    (1000000 / SCHED_CTRL_HZ)
#define SCHED_IMU_PERIOD_US     (1000000 / SCHED_IMU_HZ)
#define SCHED_BARO_PERIOD_US    (1000000 / SCHED_BARO_HZ)
#define SCHED_MAG_PERIOD_US     (1000000 / SCHED_MAG_HZ)
#define SCHED_GNSS_UART_PERIOD_US (1000000 / SCHED_GNSS_UART_HZ)
#define SCHED_RC_UART_PERIOD_US (1000000 / SCHED_RC_UART_HZ)
#define SCHED_ADC_PERIOD_US     (1000000 / SCHED_ADC_HZ)
#define SCHED_MODE_PERIOD_US    (1000000 / SCHED_MODE_HZ)
#define SCHED_MAVLINK_RX_PERIOD_US (1000000 / SCHED_MAVLINK_RX_HZ)
#define SCHED_MAVLINK_TX_PERIOD_US (1000000 / SCHED_MAVLINK_TX_HZ)
#define SCHED_LOGGER_PERIOD_US  (1000000 / SCHED_LOGGER_HZ)

/* Telemetry message intervals (Hz) */
#define TELEM_HEARTBEAT_HZ      1
#define TELEM_ATTITUDE_HZ       10
#define TELEM_GLOBAL_POS_HZ     5
#define TELEM_VFR_HUD_HZ        5
#define TELEM_SYS_STATUS_HZ     2
#define TELEM_GPS_RAW_HZ        5
#define TELEM_RC_CHANNELS_HZ    10
#define TELEM_SERVO_OUTPUT_HZ   10

/* Scheduler limits */
#define SCHED_MAX_TASKS         16
#define SCHED_MAX_OVERRUNS      1000

/* Vehicle state limits */
#define MISSION_MAX_ITEMS       100
#define PARAM_MAX_COUNT         128
#define PARAM_NAME_LEN          16
#define LOG_BUFFER_SIZE         (16 * 1024)
#define UART_TX_RING_SIZE       (4 * 1024)
#define UART_RX_RING_SIZE       (1 * 1024)

/* Failsafe defaults (overridable by MAVLink parameters) */
#define FS_BATT_LOW_MV_DEFAULT      14800   /* 3.7V per cell * 4S */
#define FS_BATT_CRIT_MV_DEFAULT     13200   /* 3.3V per cell * 4S */
#define FS_BATT_DEBOUNCE_MS_DEFAULT 2000
#define FS_RC_TIMEOUT_MS_DEFAULT    1500
#define FS_GCS_TIMEOUT_MS_DEFAULT   5000
#define FS_GNSS_TIMEOUT_MS_DEFAULT  5000

/* Arming defaults */
#define ARM_MIN_BATTERY_MV       14800
#define ARM_THROTTLE_MAX         100      /* Throttle must be below this (0-1000) */

/* Mission defaults */
#define WP_RADIUS_M_DEFAULT      2.0f
#define WP_ALT_TOLERANCE_M_DEFAULT 1.0f
#define RTL_ALT_M_DEFAULT        15.0f
#define RTL_LOITER_S_DEFAULT     5
#define LAND_SPEED_MPS_DEFAULT   0.5f

/* Control stub defaults (placeholder) */
#define CTRL_STUB_MAX_THROTTLE   1000

/* QSPI flash geometry + timing (must match emulator src/qspi.rs) */
#define QSPI_SECTOR_SIZE          4096    /* 4 KB erase sectors */
#define QSPI_PAGE_SIZE            256     /* 256 B program pages */
#define QSPI_PAGE_PROGRAM_US      500     /* ~0.5 ms @ 100 MHz (emulator: 50k cycles) */
#define QSPI_SECTOR_ERASE_MS      200     /* ~200 ms @ 100 MHz (emulator: 20M cycles) */

/* MAVLink */
#define MAVLINK_SYSID           1
#define MAVLINK_COMPID          1
#define MAVLINK_MAX_PAYLOAD_LEN 255

/* Build-time feature flags */
#define ENABLE_PROFILING        1
#define ENABLE_LOGGING          1
#define ENABLE_MAVPROXY         1

#endif /* CONFIG_H */