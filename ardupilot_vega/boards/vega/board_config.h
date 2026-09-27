/**
 * @file    board_config.h
 * @brief   VEGA (ARIES v3) Board Configuration for ArduPilot/ChibiOS
 *
 * Configuration for VEGA processor (RV32IMAC) on ARIES v3 development board.
 * Based on NIDAR VEGAPilot mission brief requirements.
 *
 * All values marked TODO must be filled from VEGA datasheet and ARIES v3 schematic.
 */

#pragma once

/*
 * Board identifier - used by build system
 */
#define BOARD_NAME                    "VEGA"
#define BOARD_VERSION                 1

/*
 * CPU & Architecture Configuration
 */
#define CPU_TYPE                      "VEGA"
#define CPU_ARCH                      "RISCV"
#define CPU_VARIANT                   "RV32IMAC"    // RV32I + M + A + C extensions

/*
 * Clock Configuration
 * TODO: Verify all clock frequencies from VEGA datasheet
 */
#define SYSCLK_FREQ                   100000000     // System clock: 100 MHz (TODO: verify)
#define APB1_FREQ                     50000000      // APB1 bus: 50 MHz (TODO: verify)
#define APB2_FREQ                     50000000      // APB2 bus: 50 MHz (TODO: verify)

/* Crystal / Oscillator Configuration */
#define HSE_FREQ                      24000000      // External crystal: 24 MHz (TODO: verify)
#define LSE_FREQ                      32768         // Low-speed external: 32.768 kHz
#define HSI_FREQ                      8000000       // Internal RC: 8 MHz (if available)
#define LSI_FREQ                      32000         // Internal low-speed: 32 kHz (if available)

/* PLL Configuration (if VEGA has PLL) */
#define PLL_SOURCE                    HSE_FREQ      // PLL source: HSE
#define PLL_MUL                       4             // PLL multiplier (100MHz = 24MHz * 4 + 4MHz)
#define PLL_DIV                       1             // PLL divider

/* Flash Wait States (for 100MHz) */
#define FLASH_WAIT_STATES             3             // 3 wait states for 100MHz (TODO: verify)

/*
 * Memory Map Configuration
 * VEGA has no internal flash - boots from external QSPI
 */
#define FLASH_BASE                    0x00000000    // QSPI flash (XIP)
#define FLASH_SIZE                    0x1000000     // 16 MB QSPI flash

#define SRAM_BASE                     0x80000000    // Internal SRAM
#define SRAM_SIZE                     0x80000       // 512 KB SRAM

#define PERIPH_BASE                   0x10000000    // Peripheral base address

/* Boot Configuration */
#define BOOT_FROM_QSPI                1             // Boot from QSPI flash
#define BOOTLOADER_SIZE               0x8000        // 32 KB bootloader
#define APPLICATION_BASE              (FLASH_BASE + BOOTLOADER_SIZE)

/*
 * Interrupt Controller Configuration
 * VEGA uses RISC-V PLIC (Platform-Level Interrupt Controller)
 */
#define PLIC_BASE                     0x0C000000    // PLIC base address
#define PLIC_NUM_INTERRUPTS           64            // Number of interrupt sources
#define PLIC_NUM_PRIORITIES           8             // Priority levels (0-7)
#define PLIC_NUM_HARTS                1             // Number of harts

/* RISC-V CLINT (Core Local Interruptor) */
#define CLINT_BASE                    0x02000000    // CLINT base address
#define CLINT_MTIMECMP_OFFSET         0x4000        // MTIMECMP offset
#define CLINT_MTIME_OFFSET            0xBFF8        // MTIME offset

/*
 * Peripheral Base Addresses (from hwdef.dat memory map)
 */
#define UART0_BASE                    0x10000000    // GCS Telemetry
#define UART1_BASE                    0x10001000    // GNSS Module
#define UART2_BASE                    0x10002000    // RC Receiver
#define UART3_BASE                    0x10003000    // Debug Console (if available)

#define SPI0_BASE                     0x10030000    // IMU (MPU6000/ICM20602)
#define SPI1_BASE                     0x10031000    // Secondary SPI (Flash/SD)

#define I2C0_BASE                     0x10050000    // Baro (MS5611) + Compass (HMC5883L)
#define I2C1_BASE                     0x10051000    // GPS (u-blox)
#define I2C2_BASE                     0x10052000    // Optional I2C

#define PWM_BASE                      0x10090000    // 8-channel PWM (ESC + Servo)

#define ADC_BASE                      0x10080000    // 8-channel ADC

#define GPIO_BASE                     0x10010000    // GPIO (LED, Buzzer, Safety)

#define TIMER_BASE                    0x10020000    // General purpose timers

#define DMA_BASE                      0x10060000    // DMA controller

#define QSPI_BASE                     0x40000000    // QSPI Flash (XIP)

#define DMA_BASE                      0x10060000
#define WDT_BASE                      0x10070000

/*
 * UART Configuration
 */
#define UART_NUM                      4             // Number of UARTs
#define UART_TX_FIFO_SIZE             64            // TX FIFO depth
#define UART_RX_FIFO_SIZE             64            // RX FIFO depth

/* Default baud rates */
#define CONSOLE_BAUD                  115200        // Debug console
#define TELEMETRY_BAUD                115200        // GCS telemetry
#define GNSS_BAUD                     115200        // GNSS module
#define RC_BAUD                       115200        // RC receiver

/* UART DMA channels (if VEGA supports UART DMA) */
#define UART0_DMA_RX                  0             // TODO: DMA channel for UART0 RX
#define UART0_DMA_TX                  0             // TODO: DMA channel for UART0 TX
#define UART1_DMA_RX                  0             // TODO: DMA channel for UART1 RX
#define UART1_DMA_TX                  0             // TODO: DMA channel for UART1 TX

/*
 * SPI Configuration
 */
#define SPI_NUM                       2             // Number of SPI controllers

/* SPI0: IMU (MPU6000/ICM20602) */
#define SPI0_MAX_FREQ                 10000000      // 10 MHz max for IMU
#define SPI0_MODE                     0             // SPI mode 0 (CPOL=0, CPHA=0)
#define SPI0_DMA_RX                   0             // TODO: DMA channel
#define SPI0_DMA_TX                   0             // TODO: DMA channel

/* SPI1: Secondary (Flash/SD) */
#define SPI1_MAX_FREQ                 25000000      // 25 MHz max
#define SPI1_MODE                     0
#define SPI1_DMA_RX                   0
#define SPI1_DMA_TX                   0

/* SPI CS GPIO mapping (from hwdef.dat) */
#define SPI0_CS0_GPIO                 GPIO_XX       // IMU CS
#define SPI0_CS1_GPIO                 GPIO_XX       // Optional
#define SPI0_CS2_GPIO                 GPIO_XX
#define SPI0_CS3_GPIO                 GPIO_XX
#define SPI1_CS0_GPIO                 GPIO_XX
#define SPI1_CS1_GPIO                 GPIO_XX
#define SPI1_CS2_GPIO                 GPIO_XX
#define SPI1_CS3_GPIO                 GPIO_XX

/*
 * I2C Configuration
 */
#define I2C_NUM                       3             // Number of I2C buses

/* I2C0: Barometer (MS5611 @ 0x77) + Compass (HMC5883L @ 0x1E) */
#define I2C0_SPEED                    400000        // 400 kHz Fast Mode
#define I2C0_DMA_RX                   0
#define I2C0_DMA_TX                   0

/* I2C1: GPS (u-blox @ 0x42) */
#define I2C1_SPEED                    100000        // 100 kHz Standard Mode
#define I2C1_DMA_RX                   0
#define I2C1_DMA_TX                   0

/* I2C2: Optional sensors */
#define I2C2_SPEED                    100000
#define I2C2_DMA_RX                   0
#define I2C2_DMA_TX                   0

/* I2C Address Definitions */
#define I2C_ADDR_MS5611               0x77          // Barometer
#define I2C_ADDR_HMC5883L             0x1E          // Compass
#define I2C_ADDR_GPS                  0x42          // u-blox GPS

/*
 * PWM Configuration
 */
#define PWM_NUM_CHANNELS              8             // 8 channels (4 ESC + 4 servo)
#define PWM_DEFAULT_FREQ              400           // 400 Hz for ESC
#define PWM_SERVO_FREQ                50            // 50 Hz for servos
#define PWM_RESOLUTION                12            // 12-bit resolution (0-4095)
#define PWM_DEAD_TIME_NS              1000          // 1us dead time (ns)

/* PWM Output Mapping */
#define PWM_MOTOR_1                   0             // ESC 1
#define PWM_MOTOR_2                   1             // ESC 2
#define PWM_MOTOR_3                   2             // ESC 3
#define PWM_MOTOR_4                   3             // ESC 4
#define PWM_SERVO_1                   4             // Servo 1
#define PWM_SERVO_2                   5             // Servo 2
#define PWM_SERVO_3                   6             // Servo 3
#define PWM_SERVO_4                   5             // Servo 4

/* DShot Configuration */
#define DSHOT_TYPE                    600           // DShot600
#define DSHOT_BIDIR                   0             // Bidirectional DShot (0=off, 1=on)

/*
 * ADC Configuration
 */
#define ADC_NUM_CHANNELS              8             // 8 ADC channels
#define ADC_RESOLUTION                12            // 12-bit ADC
#define ADC_MAX_FREQ                  1000000       // 1 MHz max sample rate

/* ADC Channel Mapping */
#define ADC_CH_BATTERY                0             // Battery voltage
#define ADC_CH_CURRENT                1             // Current sensor
#define ADC_CH_RSSI                   2             // RSSI
#define ADC_CH_SPARE_3                3
#define ADC_CH_SPARE_4                4
#define ADC_CH_SPARE_5                5
#define ADC_CH_SPARE_6                5
#define ADC_CH_SPARE_7                7

/* Battery Monitor Calibration */
#define BATTERY_VOLTAGE_PIN           0             // ADC channel for battery
#define CURRENT_SENSOR_PIN            1             // ADC channel for current
#define BATTERY_VOLTAGE_MULT          12.3          // Voltage divider ratio (TODO: calibrate)
#define CURRENT_AMP_PER_VOLT          20.0          // Current sensor scaling (A/V)
#define CURRENT_OFFSET                0.0           // Zero current offset

/*
 * GPIO Configuration
 */
#define GPIO_NUM_LEDS                 3             // Status LEDs
#define GPIO_LED_1                    GPIO_XX       // Green - System OK
#define GPIO_LED_2                    GPIO_XX       // Blue - GPS/Comms
#define GPIO_LED_3                    GPIO_XX       // Red - Error/Warning

#define GPIO_SAFETY_SWITCH            GPIO_XX       // Safety switch (active low)
#define GPIO_BUZZER                   GPIO_XX       // Buzzer output

#define GPIO_SPI0_CS0                 GPIO_XX       // IMU CS
#define GPIO_SPI0_CS1                 GPIO_XX       // Spare CS

#define GPIO_I2C0_SCL                 GPIO_XX       // I2C0 SCL (bit-bang recovery)
#define GPIO_I2C0_SDA                 GPIO_XX       // I2C0 SDA (bit-bang recovery)

/* External Interrupts */
#define EXT_INT_IMU_DRDY              GPIO_XX       // IMU Data Ready
#define EXT_INT_SAFETY                GPIO_XX       // Safety switch interrupt
#define EXT_INT_RC_1                  GPIO_XX       // RC Channel 1
#define EXT_INT_RC_2                  GPIO_XX       // RC Channel 2
#define EXT_INT_RC_3                  GPIO_XX       // RC Channel 3
#define EXT_INT_RC_4                  GPIO_XX       // RC Channel 4
#define EXT_INT_RC_5                  GPIO_XX       // RC Channel 5
#define EXT_INT_RC_6                  GPIO_XX       // RC Channel 6
#define EXT_INT_RC_7                  GPIO_XX       // RC Channel 7
#define EXT_INT_RC_8                  GPIO_XX       // RC Channel 8

/*
 * QSPI Flash Configuration
 */
#define QSPI_FREQ                     50000000      // 50 MHz QSPI clock
#define QSPI_MODE                     4             // Quad SPI mode
#define QSPI_CS_GPIO                  GPIO_XX       // QSPI CS
#define QSPI_SCK_GPIO                 GPIO_XX       // QSPI SCK
#define QSPI_MOSI_GPIO                GPIO_XX       // QSPI IO0
#define QSPI_MISO_GPIO                GPIO_XX       // QSPI IO1
#define QSPI_IO2_GPIO                 GPIO_XX       // QSPI IO2
#define QSPI_IO3_GPIO                 GPIO_XX       // QSPI IO3

#define QSPI_XIP_BASE                 0x00000000    // XIP base address
#define QSPI_XIP_SIZE                 0x1000000     // 16 MB

/* SD Card (if available) */
#define SDCARD_SPI                    1             // Use SPI1 for SD
#define SDCARD_CS_GPIO                GPIO_XX       // SD card CS
#define SDCARD_DETECT_GPIO            GPIO_XX       // SD card detect

/*
 * Power Management
 */
#define POWER_BRICK_VALID_GPIO        GPIO_XX       // Power brick valid signal
#define POWER_BRICK_CURRENT_ADC       ADC_XX        // Current ADC channel
#define POWER_BRICK_VOLTAGE_ADC       ADC_XX        // Voltage ADC channel

/*
 * Bootloader Configuration
 */
#define BOOTLOADER_ADDRESS            0x00000000    // Bootloader at flash start
#define BOOTLOADER_SIZE               0x8000        // 32 KB
#define APP_START_ADDRESS             (FLASH_BASE + BOOTLOADER_SIZE)

/*
 * Watchdog Configuration
 */
#define WDT_TIMEOUT_MS                2000          // 2 second watchdog
#define WDT_WINDOW_MS                 100           // Window watchdog (optional)

/*
 * RTOS Configuration (ChibiOS)
 */
#define CH_CFG_ST_RESOLUTION          1000000       // System tick: 1 MHz (1us resolution)
#define CH_CFG_TIME_QUANTUM           10            // Round-robin time quantum (ms)
#define CH_CFG_USE_MUTEXES            TRUE
#define CH_CFG_USE_SEMAPHORES         TRUE
#define CH_CFG_USE_EVENTS             TRUE
#define CH_CFG_USE_MESSAGES           TRUE
#define CH_CFG_USE_MAILBOXES          TRUE
#define CH_CFG_USE_DYNAMIC            TRUE

/* Stack Sizes (bytes) */
#define IDLE_THREAD_STACK_SIZE        1024
#define MAIN_THREAD_STACK_SIZE        4096
#define TELEMETRY_THREAD_STACK_SIZE   2048
#define SENSOR_THREAD_STACK_SIZE      2048
#define CONTROL_THREAD_STACK_SIZE     4096
#define MISSION_THREAD_STACK_SIZE     2048

/* Thread Priorities (higher = more important) */
#define PRIO_IDLE                     0
#define PRIO_LOW                      1
#define PRIO_NORMAL                   64
#define PRIO_HIGH                     128
#define PRIO_TELEMETRY                100
#define PRIO_SENSORS                  110
#define PRIO_CONTROL                  120
#define PRIO_MISSION                  90
#define PRIO_CRITICAL                 200

/*
 * MAVLink Configuration
 */
#define MAVLINK_SYS_ID                1             // System ID
#define MAVLINK_COMP_ID               1             // Component ID
#define MAVLINK_MAX_PAYLOAD           255
#define MAVLINK_MAX_DIALECT_PAYLOAD   255

/* MAVLink Streams (Hz) */
#define MAVLINK_STREAM_HEARTBEAT      1
#define MAVLINK_STREAM_SYS_STATUS     1
#define MAVLINK_STREAM_ATTITUDE       10
#define MAVLINK_STREAM_POSITION       5
#define MAVLINK_STREAM_RC_CHANNELS    10
#define MAVLINK_STREAM_EXTRA1         2
#define MAVLINK_STREAM_EXTRA2         2
#define MAVLINK_STREAM_EXTRA3         2

/*
 * EKF / Sensor Fusion Configuration
 */
#define EKF_ENABLED                   1
#define EKF_GYRO_NOISE                0.01          // rad/s/sqrt(Hz)
#define EKF_ACCEL_NOISE               0.1           // m/s^2/sqrt(Hz)
#define EKF_GYRO_BIAS_NOISE           0.0001        // rad/s^2/sqrt(Hz)
#define EKF_ACCEL_BIAS_NOISE          0.001         // m/s^3/sqrt(Hz)

/* Magnetometer Calibration */
#define MAG_DECLINATION               0.0           // Magnetic declination (rad, TODO: set for location)
#define MAG_INCLINATION               0.0           // Magnetic inclination (rad)

/*
 * Safety & Failsafe Configuration
 */
#define BATTERY_CRITICAL_VOLTAGE      3.3           // Per-cell critical (V)
#define BATTERY_LOW_VOLTAGE           3.5           // Per-cell low (V)
#define BATTERY_NUM_CELLS             4             // 4S LiPo

#define FAILSAFE_LOW_BATTERY          1             // Enable low battery failsafe
#define FAILSAFE_LINK_LOSS            1             // Enable link loss failsafe
#define FAILSAFE_GNSS_LOSS            1             // Enable GNSS loss failsafe
#define FAILSAFE_UNSTABLE             1             // Enable unstable flight failsafe

#define FAILSAFE_LINK_LOSS_TIMEOUT    2000          // ms before link loss failsafe
#define FAILSAFE_GNSS_LOSS_TIMEOUT    5000          // ms before GNSS loss failsafe

#define RTL_ALTITUDE                  15.0          // Return-to-launch altitude (m)
#define RTL_LOITER_TIME               5000          // Loiter time at RTL point (ms)

/*
 * Logging Configuration
 */
#define LOG_ENABLED                   1
#define LOG_BUFFER_SIZE               8192          // Log buffer size (bytes)
#define LOG_WRITE_INTERVAL            100           // ms between writes
#define LOG_MAX_FILE_SIZE             1048576       // 1 MB per log file

/*
 * Build-Time Feature Flags
 */
#define USE_DSHOT                     1             // Enable DShot output
#define USE_UAVCAN                    0             // UAVCAN support (not yet)
#define USE_OSD                       0             // On-Screen Display
#define USE_CAMERA                    0             // Camera trigger
#define USE_COMPANION                 0             // Companion computer link
#define USE_AIRSPEED                  0             // Airspeed sensor (fixed-wing)
#define USE_RANGEFINDER               0             // Rangefinder
#define USE_OPTICAL_FLOW              0             // Optical flow

/*
 * Debug Configuration
 */
#define DEBUG_ENABLED                 1
#define DEBUG_CONSOLE_UART            3             // UART3 for debug console
#define DEBUG_BAUD                    115200
#define ASSERT_ENABLED                1
#define TRACE_ENABLED                 1

/*
 * Version Information
 */
#define FIRMWARE_VERSION_MAJOR        0
#define FIRMWARE_VERSION_MINOR        1
#define FIRMWARE_VERSION_PATCH        0
#define FIRMWARE_VERSION_STRING       "VEGA-0.1.0"
#define HARDWARE_VERSION_STRING       "ARIES-v3-VEGA"

#endif /* BOARD_CONFIG_H */