#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define GPIO_DIR         (*((volatile unsigned int *)0x10010000))
#define GPIO_DATA        (*((volatile unsigned int *)0x10010004))

// Motor limits (0 - 255)
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

// -------------------------------------------------------------
// Quad-X Motor Mixer
// -------------------------------------------------------------

typedef struct {
    unsigned char m1; // Rear Right (CW)
    unsigned char m2; // Front Right (CCW)
    unsigned char m3; // Rear Left (CCW)
    unsigned char m4; // Front Left (CW)
} motor_mix_t;

motor_mix_t mix_quad_x(int throttle, int roll, int pitch, int yaw) {
    motor_mix_t motors;

    if (throttle < MOTOR_IDLE) {
        // Disarmed or cut throttle
        motors.m1 = MOTOR_MIN;
        motors.m2 = MOTOR_MIN;
        motors.m3 = MOTOR_MIN;
        motors.m4 = MOTOR_MIN;
        return motors;
    }

    int raw_m1 = throttle - pitch - roll + yaw;
    int raw_m2 = throttle + pitch - roll - yaw;
    int raw_m3 = throttle - pitch + roll - yaw;
    int raw_m4 = throttle + pitch + roll + yaw;

    motors.m1 = (unsigned char)clamp(raw_m1, MOTOR_IDLE, MOTOR_MAX);
    motors.m2 = (unsigned char)clamp(raw_m2, MOTOR_IDLE, MOTOR_MAX);
    motors.m3 = (unsigned char)clamp(raw_m3, MOTOR_IDLE, MOTOR_MAX);
    motors.m4 = (unsigned char)clamp(raw_m4, MOTOR_IDLE, MOTOR_MAX);

    return motors;
}

void send_motor_telemetry(const motor_mix_t *motors) {
    // Pack 4 channels into 32-bit GPIO register
    unsigned int packed = ((unsigned int)motors->m4 << 24) |
                          ((unsigned int)motors->m3 << 16) |
                          ((unsigned int)motors->m2 << 8)  |
                          ((unsigned int)motors->m1);

    // Write to hardware register (Triggers IPC socket transmission)
    GPIO_DATA = packed;
}

int main(void) {
    uart_puts("\r\n====================================================\n");
    uart_puts("   THEJAS32 Quadcopter Motor Mixer Telemetry Test   \n");
    uart_puts("====================================================\n");

    // Configure all 32 GPIO pins as outputs
    GPIO_DIR = 0xFFFFFFFF;

    // Simulation flight phases
    struct FlightCommand {
        const char *description;
        int throttle;
        int roll;
        int pitch;
        int yaw;
    } flight_plan[] = {
        {"1. Idle on Ground",         30,    0,   0,   0},
        {"2. Stable Hover (50% Thr)", 128,   0,   0,   0},
        {"3. Pitch Forward (Nose Down)", 140, 0,  35,   0},
        {"4. Fast Roll Right Turn",   150,  40,   0, -25},
        {"5. Throttle Cut (Motors Off)", 0,  0,   0,   0}
    };

    for (int i = 0; i < 5; i++) {
        uart_puts("\n[STAGE] ");
        uart_puts(flight_plan[i].description);
        uart_puts("\n");

        motor_mix_t out = mix_quad_x(
            flight_plan[i].throttle,
            flight_plan[i].roll,
            flight_plan[i].pitch,
            flight_plan[i].yaw
        );

        send_motor_telemetry(&out);

        uart_puts("  Outputs -> M1(RR): "); print_int(out.m1);
        uart_puts(" | M2(FR): "); print_int(out.m2);
        uart_puts(" | M3(RL): "); print_int(out.m3);
        uart_puts(" | M4(FL): "); print_int(out.m4);
        uart_puts("\n");

        // Small delay loop between state transitions
        for (volatile int d = 0; d < 500000; d++);
    }

    uart_puts("\n>>> MOTOR MIXER BROADCAST COMPLETE <<<\n");
    return 0;
}