/*
 * VEGA Pilot Firmware - SPI HAL (Emulator Backend)
 * Used for IMU (MPU-6000/ICM-20602)
 */

#include "config.h"
#include "types.h"
#include "hal/imu.h"
#include "hal/time.h"
#include "hal_vega/regs.h"

static inline void mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)addr = val;
}

static inline uint32_t mmio_read32(uint32_t addr) {
    return *(volatile uint32_t *)addr;
}

static inline uint8_t mmio_read8(uint32_t addr) {
    return *(volatile uint8_t *)addr;
}

static inline void mmio_write8(uint32_t addr, uint8_t val) {
    *(volatile uint8_t *)addr = val;
}

bool hal_imu_init(void) {
    /* Enable SPI controller */
    mmio_write32(SPI_CTRL, SPI_CTRL_EN);

    /* Reset IMU: write 0x80 to PWR_MGMT_1 (0x6B) */
    mmio_write8(SPI_CS, 0x00);  /* CS low */
    mmio_write8(SPI_DATA, 0x6B);  /* Register address */
    mmio_write8(SPI_DATA, 0x80);  /* Reset */
    mmio_write8(SPI_CS, 0x01);  /* CS high */

    hal_delay_ms(10);

    /* Configure: sample rate divider, gyro/accel config */
    mmio_write8(SPI_CS, 0x00);
    mmio_write8(SPI_DATA, 0x19);  /* SMPLRT_DIV */
    mmio_write8(SPI_DATA, 0x00);  /* 1 kHz */
    mmio_write8(SPI_CS, 0x01);

    mmio_write8(SPI_CS, 0x00);
    mmio_write8(SPI_DATA, 0x1B);  /* GYRO_CONFIG */
    mmio_write8(SPI_DATA, 0x18);  /* +/- 2000 dps */
    mmio_write8(SPI_CS, 0x01);

    mmio_write8(SPI_CS, 0x00);
    mmio_write8(SPI_DATA, 0x1C);  /* ACCEL_CONFIG */
    mmio_write8(SPI_DATA, 0x00);  /* +/- 2g */
    mmio_write8(SPI_CS, 0x01);

    /* Verify WHO_AM_I */
    mmio_write8(SPI_CS, 0x00);
    mmio_write8(SPI_DATA, 0x75 | 0x80);  /* Read WHO_AM_I */
    uint8_t whoami = mmio_read8(SPI_DATA);
    mmio_write8(SPI_CS, 0x01);

    return whoami == 0x68 || whoami == 0x70;  /* MPU-6000 or ICM-20602 */
}

bool hal_imu_read(imu_raw_t *out) {
    /* Burst read from ACCEL_XOUT_H (0x3B) - 14 bytes */
    mmio_write8(SPI_CS, 0x00);
    mmio_write8(SPI_DATA, 0x3B | 0x80);  /* Read burst from 0x3B */

    uint8_t buf[14];
    for (int i = 0; i < 14; i++) {
        buf[i] = mmio_read8(SPI_DATA);
    }
    mmio_write8(SPI_CS, 0x01);

    out->ax = (int16_t)((buf[0] << 8) | buf[1]);
    out->ay = (int16_t)((buf[2] << 8) | buf[3]);
    out->az = (int16_t)((buf[4] << 8) | buf[5]);
    out->gx = (int16_t)((buf[8] << 8) | buf[9]);
    out->gy = (int16_t)((buf[10] << 8) | buf[11]);
    out->gz = (int16_t)((buf[12] << 8) | buf[13]);
    out->t_us = hal_time_us();

    return true;
}