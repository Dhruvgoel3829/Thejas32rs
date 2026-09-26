#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define GPIO_DIR         (*((volatile unsigned int *)0x10010000))
#define GPIO_DATA        (*((volatile unsigned int *)0x10010004))

#define SPI_DATA         (*((volatile unsigned int *)0x10030004))
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
    if (val < 0) { uart_putc('-'); val = -val; }
    if (val == 0) { uart_putc('0'); return; }
    while (val > 0) { buf[i++] = '0' + (val % 10); val /= 10; }
    while (i > 0) { uart_putc(buf[--i]); }
}

void print_hex(unsigned char val) {
    char hex[] = "0123456789ABCDEF";
    uart_putc(hex[(val >> 4) & 0xF]);
    uart_putc(hex[val & 0xF]);
}

static inline void mpu_select(void)   { SPI_CS = 0x00; }
static inline void mpu_deselect(void) { SPI_CS = 0x01; }

unsigned char spi_transfer(unsigned char byte) {
    SPI_DATA = byte;
    return (unsigned char)(SPI_DATA & 0xFF);
}

void mpu_write_reg(unsigned char reg, unsigned char val) {
    mpu_select();
    spi_transfer(reg & 0x7F);
    spi_transfer(val);
    mpu_deselect();
}

unsigned char mpu_read_reg(unsigned char reg) {
    mpu_select();
    spi_transfer(reg | 0x80);
    unsigned char val = spi_transfer(0x00);
    mpu_deselect();
    return val;
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
    for (int i = 0; i < 14; i++) buf[i] = spi_transfer(0x00);
    mpu_deselect();

    imu->ax = (short)((buf[0] << 8) | buf[1]);
    imu->ay = (short)((buf[2] << 8) | buf[3]);
    imu->az = (short)((buf[4] << 8) | buf[5]);
    imu->temp = (short)((buf[6] << 8) | buf[7]);
    imu->gx = (short)((buf[8] << 8) | buf[9]);
    imu->gy = (short)((buf[10] << 8) | buf[11]);
    imu->gz = (short)((buf[12] << 8) | buf[13]);
}

void test_motor_sequence(void) {
    uart_puts("\n=== MOTOR SEQUENCE TEST ===\n");
    GPIO_DIR = 0xFFFFFFFF;
    
    for (int m = 0; m < 4; m++) {
        uart_puts("Motor "); print_int(m+1); uart_puts(" ON\n");
        GPIO_DATA = (1 << (m * 8)) * 100;
        for (volatile int d = 0; d < 500000; d++);
    }
    
    uart_puts("All motors 50%\n");
    GPIO_DATA = 0x80808080;
    for (volatile int d = 0; d < 1000000; d++);
    
    uart_puts("All motors 100%\n");
    GPIO_DATA = 0xFFFFFFFF;
    for (volatile int d = 0; d < 1000000; d++);
    
    uart_puts("All motors OFF\n");
    GPIO_DATA = 0;
}

void test_imu_readings(void) {
    uart_puts("\n=== IMU READINGS TEST ===\n");
    
    mpu_write_reg(MPU_REG_PWR_MGMT_1, 0x01);
    mpu_write_reg(MPU_REG_GYRO_CONFIG, 0x18);
    mpu_write_reg(MPU_REG_ACCEL_CONFIG, 0x00);
    
    unsigned char whoami = mpu_read_reg(MPU_REG_WHOAMI);
    uart_puts("WHO_AM_I: 0x"); print_hex(whoami); uart_puts("\n");
    
    imu_raw_t imu;
    for (int i = 0; i < 10; i++) {
        mpu_read_burst(&imu);
        uart_puts("AX:"); print_int(imu.ax);
        uart_puts(" AY:"); print_int(imu.ay);
        uart_puts(" AZ:"); print_int(imu.az);
        uart_puts(" GX:"); print_int(imu.gx);
        uart_puts(" GY:"); print_int(imu.gy);
        uart_puts(" GZ:"); print_int(imu.gz);
        uart_puts("\n");
        for (volatile int d = 0; d < 200000; d++);
    }
}

void test_hover_sequence(void) {
    uart_puts("\n=== HOVER SEQUENCE TEST ===\n");
    GPIO_DIR = 0xFFFFFFFF;
    
    uart_puts("Ramp up...\n");
    for (int t = 50; t <= 200; t += 10) {
        unsigned char val = (unsigned char)t;
        GPIO_DATA = (val << 24) | (val << 16) | (val << 8) | val;
        uart_puts("Throttle: "); print_int(t); uart_puts("\n");
        for (volatile int d = 0; d < 300000; d++);
    }
    
    uart_puts("Hold hover...\n");
    for (int i = 0; i < 20; i++) {
        imu_raw_t imu;
        mpu_read_burst(&imu);
        uart_puts("GX:"); print_int(imu.gx/16); uart_puts(" GY:"); print_int(imu.gy/16); uart_puts("\n");
        for (volatile int d = 0; d < 300000; d++);
    }
    
    uart_puts("Ramp down...\n");
    for (int t = 200; t >= 0; t -= 20) {
        unsigned char val = (unsigned char)t;
        GPIO_DATA = (val << 24) | (val << 16) | (val << 8) | val;
        for (volatile int d = 0; d < 200000; d++);
    }
    GPIO_DATA = 0;
}

int main(void) {
    uart_puts("\n====================================================\n");
    uart_puts("   THEJAS32 DRONE SYSTEM TEST\n");
    uart_puts("====================================================\n");
    
    test_imu_readings();
    test_motor_sequence();
    test_hover_sequence();
    
    uart_puts("\n=== ALL TESTS COMPLETE ===\n");
    
    while (1) {
        for (volatile int d = 0; d < 1000000; d++);
    }
}