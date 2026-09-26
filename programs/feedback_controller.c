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

// Simple sin/cos using lookup table (no math.h needed)
static inline int sintab(int x) {
    static const int table[64] = {
        0, 1608, 3217, 4824, 6423, 8008, 9574, 11116,
        12629, 14110, 15553, 16954, 18310, 19616, 20869, 22066,
        23204, 24280, 25290, 26230, 27099, 27892, 28607, 29241,
        29793, 30260, 30640, 30931, 31131, 31238, 31250, 31167,
        30988, 30713, 30341, 29871, 29304, 28638, 27876, 27016,
        26059, 25007, 23860, 22621, 21292, 19875, 18373, 16788,
        15123, 13382, 11568, 9686, 7741, 5737, 3680, 1576
    };
    int idx = (x >> 10) & 63;
    if (idx < 16) return table[idx];
    if (idx < 32) return table[31 - idx];
    if (idx < 48) return -table[idx - 32];
    return -table[63 - idx];
}

#define SIN(x) sintab(x)
#define COS(x) sintab((x) + 16384)

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

// Complementary filter for attitude estimation
typedef struct {
    int roll;
    int pitch;
    int yaw;
    int roll_rate;
    int pitch_rate;
    int yaw_rate;
    unsigned int last_update;
} attitude_t;

void attitude_init(attitude_t *att) {
    att->roll = 0;
    att->pitch = 0;
    att->yaw = 0;
    att->roll_rate = 0;
    att->pitch_rate = 0;
    att->yaw_rate = 0;
    att->last_update = 0;
}

// Alpha = 0.98 for complementary filter (98% gyro, 2% accel)
#define ALPHA_NUM 98
#define ALPHA_DEN 100

void attitude_update(attitude_t *att, const imu_raw_t *imu, unsigned int dt_ms) {
    // Gyro in raw LSB -> degrees per second
    // MPU-6000 at +/-2000 dps: 16.4 LSB/dps
    int gx_dps = imu->gx / 16;
    int gy_dps = imu->gy / 16;
    int gz_dps = imu->gz / 16;
    
    att->roll_rate = gx_dps;
    att->pitch_rate = gy_dps;
    att->yaw_rate = gz_dps;
    
    // Integrate gyro (dt in ms -> convert to seconds)
    int dt_s = dt_ms / 1000;  // Simplified: assume 1ms loop
    if (dt_s == 0) dt_s = 1;
    
    att->roll  += (att->roll_rate * dt_s) / 1000;
    att->pitch += (att->pitch_rate * dt_s) / 1000;
    att->yaw   += (att->yaw_rate * dt_s) / 1000;
    
    // Accel-based roll/pitch (in degrees * 100)
    // Use atan2 approximation: roll = atan2(ay, az), pitch = atan2(-ax, az)
    // For small angles: roll ≈ ay/az * 5730 (deg*100), pitch ≈ -ax/az * 5730
    int az_safe = imu->az;
    if (az_safe < 100) az_safe = 100;  // Prevent division by zero/small
    if (az_safe > 32000) az_safe = 32000;
    
    int accel_roll  = ((int)imu->ay * 5730) / az_safe;
    int accel_pitch = ((int)-imu->ax * 5730) / az_safe;
    
    // Clamp accel angles to reasonable range
    if (accel_roll > 9000) accel_roll = 9000;
    if (accel_roll < -9000) accel_roll = -9000;
    if (accel_pitch > 9000) accel_pitch = 9000;
    if (accel_pitch < -9000) accel_pitch = -9000;
    
    // Complementary filter (alpha = 0.98)
    att->roll  = (ALPHA_NUM * att->roll  + (ALPHA_DEN - ALPHA_NUM) * accel_roll)  / ALPHA_DEN;
    att->pitch = (ALPHA_NUM * att->pitch + (ALPHA_DEN - ALPHA_NUM) * accel_pitch) / ALPHA_DEN;
}

typedef struct {
    int kp;
    int ki;
    int kd;
    int integral;
    int prev_error;
    int max_integral;
} pid_t;

void pid_init(pid_t *pid, int kp, int ki, int kd, int max_int) {
    pid->kp = kp;
    pid->ki = ki;
    pid->kd = kd;
    pid->integral = 0;
    pid->prev_error = 0;
    pid->max_integral = max_int;
}

int pid_update(pid_t *pid, int target, int current) {
    int error = target - current;
    
    // P term
    int p_out = (pid->kp * error) / 100;
    
    // I term
    pid->integral += error;
    if (pid->integral > pid->max_integral) pid->integral = pid->max_integral;
    if (pid->integral < -pid->max_integral) pid->integral = -pid->max_integral;
    int i_out = (pid->ki * pid->integral) / 100;
    
    // D term
    int derivative = error - pid->prev_error;
    pid->prev_error = error;
    int d_out = (pid->kd * derivative) / 100;
    
    return p_out + i_out + d_out;
}

static inline int clamp(int val, int min, int max) {
    if (val < min) return min;
    if (val > max) return max;
    return val;
}

void write_motors(int m1, int m2, int m3, int m4) {
    m1 = clamp(m1, 0, 255);
    m2 = clamp(m2, 0, 255);
    m3 = clamp(m3, 0, 255);
    m4 = clamp(m4, 0, 255);
    
    GPIO_DATA = ((unsigned int)m4 << 24) |
                ((unsigned int)m3 << 16) |
                ((unsigned int)m2 << 8) |
                ((unsigned int)m1);
}

int main(void) {
    GPIO_DIR = 0xFFFFFFFF;
    
    uart_puts("\n====================================================\n");
    uart_puts("   THEJAS32 FEEDBACK-CONTROLLED DRONE\n");
    uart_puts("====================================================\n");
    
    // Init MPU-6000
    mpu_write_reg(MPU_REG_PWR_MGMT_1, 0x01);   // Wake up, PLL with X gyro
    mpu_write_reg(MPU_REG_GYRO_CONFIG, 0x18);  // +/-2000 dps
    mpu_write_reg(MPU_REG_ACCEL_CONFIG, 0x00); // +/-2g
    
    unsigned char whoami = mpu_read_reg(MPU_REG_WHOAMI);
    uart_puts("MPU WHO_AM_I: 0x");
    char hex[] = "0123456789ABCDEF";
    uart_putc(hex[(whoami >> 4) & 0xF]);
    uart_putc(hex[whoami & 0xF]);
    uart_puts("\n");
    
    if (whoami != 0x68) {
        uart_puts("ERROR: MPU not responding correctly! Using synthetic data.\n");
    }
    
    // Altitude PID (using accel Z as proxy for vertical position)
    pid_t alt_pid;
    pid_init(&alt_pid, 8, 2, 5, 1000);
    int target_alt = 16384;  // Hover at 1g (16384 LSB)
    int alt_throttle = 190;  // Base hover throttle
    
    // PID controllers
    pid_t roll_rate_pid;
    pid_t pitch_rate_pid;
    pid_init(&roll_rate_pid, 15, 1, 5, 1000);
    pid_init(&pitch_rate_pid, 15, 1, 5, 1000);
    
    pid_t roll_angle_pid;
    pid_t pitch_angle_pid;
    pid_init(&roll_angle_pid, 20, 0, 10, 500);
    pid_init(&pitch_angle_pid, 20, 0, 10, 500);
    
    pid_t yaw_rate_pid;
    pid_init(&yaw_rate_pid, 15, 1, 6, 500);  // Stronger yaw
    
    attitude_t att;
    attitude_init(&att);
    
    imu_raw_t imu;
    
    int target_roll = 0;
    int target_pitch = 0;
    int target_yaw = 0;  // Yaw angle hold (not rate)
    
    int synth_tick = 0;
    
    uart_puts("[FC] Starting control loop...\n\n");
    
    unsigned int cycle = 0;
    while (1) {
        cycle++;
        
        // Read IMU
        mpu_read_burst(&imu);
        
        // Fallback to synthetic data if IMU reads zero
        if (imu.ax == 0 && imu.ay == 0 && imu.az == 0 && imu.gx == 0 && imu.gy == 0 && imu.gz == 0) {
            synth_tick += 1000;  // 1000 units per cycle
            int phase = synth_tick & 0xFFFF;
            imu.ax = (short)((SIN(phase) * 100) / 32767);
            imu.ay = 0;
            imu.az = 16384;
            imu.gx = (short)((SIN(phase) * 1000) / 32767);
            imu.gy = (short)((COS(phase) * 500) / 32767);
            imu.gz = 0;
        }
        
        // Update attitude (dt ~ 1ms per loop)
        attitude_update(&att, &imu, 1);
        
        // Altitude PID (using accel Z as altitude proxy)
        // Target: 16384 = 1g (hover). Error > 0 means too low -> increase throttle
        int alt_corr = pid_update(&alt_pid, target_alt, imu.az);
        int throttle = alt_throttle + alt_corr;
        throttle = clamp(throttle, 150, 240);  // Limit throttle range
        
        // Cascade PID: Angle -> Rate -> Motor
        int roll_rate_target  = pid_update(&roll_angle_pid,  target_roll,  att.roll);
        int pitch_rate_target = pid_update(&pitch_angle_pid, target_pitch, att.pitch);
        
        int roll_corr  = pid_update(&roll_rate_pid,  roll_rate_target,  att.roll_rate);
        int pitch_corr = pid_update(&pitch_rate_pid, pitch_rate_target, att.pitch_rate);
        
        // Yaw angle hold (outer) -> Rate (inner)
        int yaw_rate_target = pid_update(&yaw_rate_pid, target_yaw, att.yaw);
        int yaw_corr   = pid_update(&yaw_rate_pid, yaw_rate_target, att.yaw_rate);
        
        // Mixer (X configuration)
        // M1=RR(CW), M2=FR(CCW), M3=RL(CCW), M4=FL(CW)
        int m1 = throttle - pitch_corr - roll_corr - yaw_corr;
        int m2 = throttle + pitch_corr - roll_corr + yaw_corr;
        int m3 = throttle + pitch_corr + roll_corr - yaw_corr;
        int m4 = throttle - pitch_corr + roll_corr + yaw_corr;
        
        write_motors(m1, m2, m3, m4);
        
        // Telemetry every 20 cycles
        if ((cycle % 20) == 0) {
            uart_puts("Loop "); print_int(cycle);
            uart_puts(" | Roll:"); print_int(att.roll);
            uart_puts(" Pitch:"); print_int(att.pitch);
            uart_puts(" Yaw:"); print_int(att.yaw);
            uart_puts(" | Rrate:"); print_int(att.roll_rate);
            uart_puts(" Prate:"); print_int(att.pitch_rate);
            uart_puts(" Yrate:"); print_int(att.yaw_rate);
            uart_puts(" | AZ:"); print_int(imu.az);
            uart_puts(" Throt:"); print_int(throttle);
            uart_puts(" | M1:"); print_int(m1);
            uart_puts(" M2:"); print_int(m2);
            uart_puts(" M3:"); print_int(m3);
            uart_puts(" M4:"); print_int(m4);
            uart_puts("\n");
        }
        
        // Test sequence: step targets - GENTLE
        if (cycle == 2000) { target_roll = 200;  uart_puts("\n>>> STEP: Roll +2deg\n"); }
        if (cycle == 4000) { target_roll = -200; uart_puts("\n>>> STEP: Roll -2deg\n"); }
        if (cycle == 6000) { target_roll = 0;     uart_puts("\n>>> STEP: Roll 0deg\n"); }
        if (cycle == 8000) { target_pitch = 200; uart_puts("\n>>> STEP: Pitch +2deg\n"); }
        if (cycle == 10000) { target_pitch = -200; uart_puts("\n>>> STEP: Pitch -2deg\n"); }
        if (cycle == 12000) { target_pitch = 0;    uart_puts("\n>>> STEP: Pitch 0deg\n"); }
        if (cycle == 14000) { target_yaw = 500; uart_puts("\n>>> STEP: Yaw +5deg\n"); }
        if (cycle == 16000) { target_yaw = -500; uart_puts("\n>>> STEP: Yaw -5deg\n"); }
        if (cycle == 18000) { target_yaw = 0;    uart_puts("\n>>> STEP: Yaw 0deg\n"); }
        
        for (volatile int d = 0; d < 8000; d++);  // ~1ms delay
    }
}