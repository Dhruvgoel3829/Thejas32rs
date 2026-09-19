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

#define MOTOR_MIN        0
#define MOTOR_IDLE       25
#define MOTOR_MAX        250

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

static inline int clamp(int val, int min, int max) {
    if (val < min) return min;
    if (val > max) return max;
    return val;
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

typedef struct {
    int kp;
    int ki;
    int kd;
    int integral;
    int prev_error;
} pid_t;

int pid_update(pid_t *pid, int target, int current) {
    int error = target - current;

    int p_out = (pid->kp * error) / 100;

    pid->integral += error;
    pid->integral = clamp(pid->integral, -500, 500);
    int i_out = (pid->ki * pid->integral) / 100;

    int derivative = error - pid->prev_error;
    pid->prev_error = error;
    int d_out = (pid->kd * derivative) / 100;

    return p_out + i_out + d_out;
}

void write_motors(int throttle, int roll_corr, int pitch_corr, int yaw_corr) {
    if (throttle < MOTOR_IDLE) {
        GPIO_DATA = 0;
        return;
    }

    int m1 = throttle - pitch_corr - roll_corr + yaw_corr;
    int m2 = throttle + pitch_corr - roll_corr - yaw_corr;
    int m3 = throttle - pitch_corr + roll_corr - yaw_corr;
    int m4 = throttle + pitch_corr + roll_corr + yaw_corr;

    unsigned char out1 = (unsigned char)clamp(m1, MOTOR_IDLE, MOTOR_MAX);
    unsigned char out2 = (unsigned char)clamp(m2, MOTOR_IDLE, MOTOR_MAX);
    unsigned char out3 = (unsigned char)clamp(m3, MOTOR_IDLE, MOTOR_MAX);
    unsigned char out4 = (unsigned char)clamp(m4, MOTOR_IDLE, MOTOR_MAX);

    GPIO_DATA = ((unsigned int)out4 << 24) |
                ((unsigned int)out3 << 16) |
                ((unsigned int)out2 << 8)  |
                ((unsigned int)out1);
}

int main(void) {
    GPIO_DIR = 0xFFFFFFFF;

    uart_puts("\r\n====================================================\n");
    uart_puts("   THEJAS32 Closed-Loop Flight Controller Running   \n");
    uart_puts("====================================================\n");

    mpu_write_reg(MPU_REG_PWR_MGMT_1, 0x01);
    mpu_write_reg(MPU_REG_GYRO_CONFIG, 0x18);
    mpu_write_reg(MPU_REG_ACCEL_CONFIG, 0x00);

    pid_t roll_pid  = { .kp = 120, .ki = 5, .kd = 40, .integral = 0, .prev_error = 0 };
    pid_t pitch_pid = { .kp = 120, .ki = 5, .kd = 40, .integral = 0, .prev_error = 0 };

    int target_roll_rate  = 0;
    int target_pitch_rate = 0;

    imu_raw_t imu;

    uart_puts("[FC] Closed-loop active. Running continuous flight loop:\n\n");

    int cycle = 0;
    while (1) {
        cycle++;

        mpu_read_burst(&imu);

        int current_roll_rate  = imu.gx / 16;
        int current_pitch_rate = imu.gy / 16;

        int roll_corr  = pid_update(&roll_pid, target_roll_rate, current_roll_rate);
        int pitch_corr = pid_update(&pitch_pid, target_pitch_rate, current_pitch_rate);

        write_motors(135, roll_corr, pitch_corr, 0);

        if ((cycle % 20) == 0) {
            uart_puts("Loop #"); print_int(cycle);
            uart_puts(" | Roll Rate: "); print_int(current_roll_rate);
            uart_puts(" dps | Roll Corr: "); print_int(roll_corr);
            uart_puts("\n");
        }

        for (volatile int d = 0; d < 8000; d++);
    }
}