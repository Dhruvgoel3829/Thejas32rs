#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define SPI_CTRL         (*((volatile unsigned int *)0x10030000))
#define SPI_DATA         (*((volatile unsigned int *)0x10030004))
#define SPI_STATUS       (*((volatile unsigned int *)0x10030008))
#define SPI_CS           (*((volatile unsigned int *)0x1003000C))

#define MPU_REG_WHOAMI       0x75
#define MPU_REG_PWR_MGMT_1   0x6B
#define MPU_REG_ACCEL_CONFIG 0x1C
#define MPU_REG_GYRO_CONFIG  0x1B
#define MPU_REG_ACCEL_XOUT_H 0x3B

void uart_putc(char c) {
    while ((UART_LSR & 0x20) == 0);
    UART_THR = c;
}

void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') uart_putc('\r');
        uart_putc(*s++);
    }
}

void print_int(int val) {
    char buf[12];
    int i = 0;
    if (val < 0) {
        uart_putc('-');
        val = -val;
    }
    if (val == 0) {
        uart_putc('0');
        return;
    }
    while (val > 0) {
        buf[i++] = '0' + (val % 10);
        val /= 10;
    }
    while (i > 0) {
        uart_putc(buf[--i]);
    }
}

static inline void mpu_select(void) {
    SPI_CS = 0x00; // Pull CS0 low
}

static inline void mpu_deselect(void) {
    SPI_CS = 0x01; // Pull CS0 high
}

unsigned char spi_transfer(unsigned char byte) {
    SPI_DATA = byte;
    return (unsigned char)(SPI_DATA & 0xFF);
}

unsigned char mpu_read_reg(unsigned char reg) {
    mpu_select();
    spi_transfer(reg | 0x80); // Read bit (0x80)
    unsigned char val = spi_transfer(0x00);
    mpu_deselect();
    return val;
}

void mpu_write_reg(unsigned char reg, unsigned char val) {
    mpu_select();
    spi_transfer(reg & 0x7F); // Write bit (0x00)
    spi_transfer(val);
    mpu_deselect();
}

typedef struct {
    short ax, ay, az;
    short temp;
    short gx, gy, gz;
} __attribute__((packed)) imu_raw_t;

void mpu_read_burst(imu_raw_t *imu) {
    unsigned char buf[14];
    mpu_select();
    spi_transfer(MPU_REG_ACCEL_XOUT_H | 0x80);
    for (int i = 0; i < 14; i++) {
        buf[i] = spi_transfer(0x00);
    }
    mpu_deselect();

    imu->ax = (short)((buf[0] << 8) | buf[1]);
    imu->ay = (short)((buf[2] << 8) | buf[3]);
    imu->az = (short)((buf[4] << 8) | buf[5]);
    imu->temp = (short)((buf[6] << 8) | buf[7]);
    imu->gx = (short)((buf[8] << 8) | buf[9]);
    imu->gy = (short)((buf[10] << 8) | buf[11]);
    imu->gz = (short)((buf[12] << 8) | buf[13]);
}

int main(void) {
    uart_puts("\r\n====================================================\n");
    uart_puts("   THEJAS32 Flight Controller SPI IMU Driver Test   \n");
    uart_puts("====================================================\n");

    // 1. Verify WHO_AM_I
    unsigned char whoami = mpu_read_reg(MPU_REG_WHOAMI);
    uart_puts("[IMU] Probing WHO_AM_I register (0x75)... Result: 0x");
    const char hex[] = "0123456789ABCDEF";
    uart_putc(hex[(whoami >> 4) & 0xF]);
    uart_putc(hex[whoami & 0xF]);
    uart_puts("\n");

    if (whoami != 0x68) {
        uart_puts("  [FAIL] Expected 0x68 (MPU-6000)\n");
        return -1;
    }
    uart_puts("  [PASS] MPU-6000 sensor detected.\n");

    // 2. Wake sensor
    mpu_write_reg(MPU_REG_PWR_MGMT_1, 0x01);
    mpu_write_reg(MPU_REG_GYRO_CONFIG, 0x18);
    mpu_write_reg(MPU_REG_ACCEL_CONFIG, 0x00);

    uart_puts("[IMU] Sensor initialized. Reading 5 live sensor bursts:\n\n");

    imu_raw_t imu_data;

    for (int i = 0; i < 5; i++) {
        for (volatile int d = 0; d < 2000; d++);

        mpu_read_burst(&imu_data);

        uart_puts("  Frame #"); print_int(i + 1);
        uart_puts(" | Accel [X,Y,Z]: [");
        print_int(imu_data.ax); uart_puts(", ");
        print_int(imu_data.ay); uart_puts(", ");
        print_int(imu_data.az);
        uart_puts("] | Gyro [X,Y,Z]: [");
        print_int(imu_data.gx); uart_puts(", ");
        print_int(imu_data.gy); uart_puts(", ");
        print_int(imu_data.gz);
        uart_puts("]\n");
    }

    uart_puts("\n>>> SPI IMU COMMUNICATION & BURST READ PASSED <<<\n");
    return 0;
}