/*
 * Cycle Budget Benchmarks
 * Measures instructions-retired (mcycle deltas) for hot-path operations.
 * Volatile inputs defeat constant folding; production flags (-Os).
 * Run on emulator, transcribe BENCH lines into docs/CYCLE_BUDGET.md.
 */

#include "config.h"
#include "types.h"
#include "hal/time.h"
#include "hal/uart.h"
#include "hal/gpio.h"
#include <stdio.h>
#include <math.h>

static volatile uint32_t vsink_u32;

static void bench_report(const char *name, uint32_t iters, uint64_t dt) {
    char buf[128];
    /* per-iter cost with 2 decimals: (dt*100)/iters. Fraction zero-padded
     * manually (%02u unsupported in minimal vsnprintf). */
    uint32_t per100 = iters ? (uint32_t)((dt * 100) / iters) : 0;
    uint32_t ip = per100 / 100, fp = per100 % 100;
    int len = snprintf(buf, sizeof(buf), "BENCH %s: %lu retired / %lu iters = %lu.%s%lu per iter\n",
        name, (unsigned long)(uint32_t)dt, (unsigned long)iters,
        (unsigned long)ip, fp < 10u ? "0" : "", (unsigned long)fp);
    hal_uart_write_nb(UART_ID_TELEM, (uint8_t *)buf, (uint16_t)len);
}

/* Volatile operand tables (normal-range values, never evolve so no
 * saturation to 0/inf across iterations). */
static volatile float ftab[8] = {0.11f, 1.7f, -2.3f, 0.707f, 3.14159f, -0.5f, 100.25f, -33.75f};
static volatile float ftab_pos[8] = {0.11f, 1.7f, 2.3f, 0.707f, 3.14159f, 0.5f, 100.25f, 33.75f};
static volatile float fsink;

/* Baseline: volatile table load + store (no ALU op) */
static void bench_copy(uint32_t n) {
    uint64_t t0 = hal_cycles();
    for (uint32_t i = 0u; i < n; i++) {
        fsink = ftab[i & 7u];
    }
    bench_report("copy_baseline", n, hal_cycles() - t0);
}

#define BENCH_BINOP(name, op, n) do { \
    uint64_t t0 = hal_cycles(); \
    for (uint32_t i = 0u; i < (n); i++) { fsink = ftab[i & 7u] op ftab[(i + 3u) & 7u]; } \
    bench_report(name, n, hal_cycles() - t0); \
} while (0)

static void bench_unary(const char *name, uint32_t n, float (*fn)(float)) {
    uint64_t t0 = hal_cycles();
    for (uint32_t i = 0u; i < n; i++) {
        fsink = fn(ftab[i & 7u]);
    }
    bench_report(name, n, hal_cycles() - t0);
}

static void bench_unary_pos(const char *name, uint32_t n, float (*fn)(float)) {
    uint64_t t0 = hal_cycles();
    for (uint32_t i = 0u; i < n; i++) {
        fsink = fn(ftab_pos[i & 7u]);
    }
    bench_report(name, n, hal_cycles() - t0);
}

/* Mahony AHRS update (IMU-only), canonical form, Kp=0.5*2 */
static void mahony_update(volatile float *q, volatile float *gyro, volatile float *acc) {
    float q0 = q[0], q1 = q[1], q2 = q[2], q3 = q[3];
    float gx = gyro[0], gy = gyro[1], gz = gyro[2];
    float ax = acc[0], ay = acc[1], az = acc[2];
    /* Normalise accelerometer */
    float recip = 1.0f / sqrtf(ax * ax + ay * ay + az * az);
    ax *= recip; ay *= recip; az *= recip;
    /* Estimated gravity direction */
    float vx = 2.0f * (q1 * q3 - q0 * q2);
    float vy = 2.0f * (q0 * q1 + q2 * q3);
    float vz = q0 * q0 - q1 * q1 - q2 * q2 + q3 * q3;
    /* Error is cross product */
    float ex = (ay * vz - az * vy);
    float ey = (az * vx - ax * vz);
    float ez = (ax * vy - ay * vx);
    /* Apply feedback (Kp = 1.0, dt folded into caller rate) */
    gx += ex; gy += ey; gz += ez;
    /* Integrate */
    const float dt = 0.001f;
    q0 += (-q1 * gx - q2 * gy - q3 * gz) * (0.5f * dt);
    q1 += (q0 * gx + q2 * gz - q3 * gy) * (0.5f * dt);
    q2 += (q0 * gy - q1 * gz + q3 * gx) * (0.5f * dt);
    q3 += (q0 * gz + q1 * gy - q2 * gx) * (0.5f * dt);
    /* Normalise quaternion */
    recip = 1.0f / sqrtf(q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3);
    q[0] = q0 * recip; q[1] = q1 * recip; q[2] = q2 * recip; q[3] = q3 * recip;
}

/* 3x3 matrix multiply: C = A*B */
static void matmul3(volatile float *c, volatile float *a, volatile float *b) {
    for (int i = 0; i < 3; i++) {
        for (int j = 0; j < 3; j++) {
            float s = 0.0f;
            for (int k = 0; k < 3; k++) {
                s += a[i * 3 + k] * b[k * 3 + j];
            }
            c[i * 3 + j] = s;
        }
    }
}

/* MAVLink v2 minimal: CRC-16/MCRF4XX accumulation */
static uint16_t mav_crc_acc(uint8_t b, uint16_t crc) {
    uint8_t t = b ^ (uint8_t)(crc & 0xFF);
    t ^= (uint8_t)(t << 4);
    return (crc >> 8) ^ ((uint16_t)t << 8) ^ ((uint16_t)t << 3) ^ ((uint16_t)t >> 4);
}

/* Pack a HEARTBEAT (msgid 0, 9 B payload, extra CRC 50) into volatile buf.
 * Returns total frame length. */
static uint32_t mav_pack_heartbeat(volatile uint8_t *frame, uint8_t seq) {
    uint8_t *f = (uint8_t *)frame;  /* drop volatile for bulk store */
    f[0] = 0xFD; f[1] = 9; f[2] = 0; f[3] = 0;
    f[4] = seq; f[5] = 1; f[6] = 1;
    f[7] = 0; f[8] = 0; f[9] = 0;  /* msgid 0 */
    /* payload: custom_mode u32 + type, autopilot, base_mode, system_status, mavlink_version */
    f[10] = 0; f[11] = 0; f[12] = 0; f[13] = 0;
    f[14] = 2; f[15] = 3; f[16] = 0x51; f[17] = 4; f[18] = 2;
    uint16_t crc = 0xFFFF;
    for (uint32_t i = 1; i < 19u; i++) {
        crc = mav_crc_acc(f[i], crc);
    }
    crc = mav_crc_acc(50, crc);  /* HEARTBEAT extra CRC */
    f[19] = (uint8_t)(crc & 0xFF);
    f[20] = (uint8_t)(crc >> 8);
    vsink_u32 = f[20];
    return 21;
}

/* Parse + verify CRC of a frame in volatile buf. Returns 1 if OK. */
static uint32_t mav_parse_verify(volatile uint8_t *frame, uint32_t len) {
    if (frame[0] != 0xFD || len < 12) {
        return 0;
    }
    uint8_t plen = frame[1];
    uint32_t msgid = frame[7] | ((uint32_t)frame[8] << 8) | ((uint32_t)frame[9] << 16);
    uint16_t crc = 0xFFFF;
    for (uint32_t i = 1; i < 10u + plen; i++) {
        crc = mav_crc_acc(frame[i], crc);
    }
    uint8_t extra = (msgid == 0) ? 50 : 0;
    crc = mav_crc_acc(extra, crc);
    uint16_t rx = frame[10 + plen] | ((uint16_t)frame[11 + plen] << 8);
    vsink_u32 = rx + msgid;
    return (crc == rx) ? 1 : 0;
}

/* Minimal NMEA GGA parse: time + lat/lon to integer microdegrees-ish */
static uint32_t nmea_parse_gga(volatile char *s) {
    /* Expect $GPGGA,hhmmss.ss,ddmm.mmmm,N,dddmm.mmmm,E,... */
    int field = 0;
    uint32_t lat = 0, lon = 0, time = 0;
    while (*s && field < 6) {
        if (*s == ',') {
            field++;
        } else if (field == 1 && *s >= '0' && *s <= '9') {
            time = time * 10 + (uint32_t)(*s - '0');
        } else if (field == 2 && ((*s >= '0' && *s <= '9') || *s == '.')) {
            if (*s != '.') {
                lat = lat * 10 + (uint32_t)(*s - '0');
            }
        } else if (field == 4 && ((*s >= '0' && *s <= '9') || *s == '.')) {
            if (*s != '.') {
                lon = lon * 10 + (uint32_t)(*s - '0');
            }
        }
        s++;
    }
    vsink_u32 = lat + lon + time;
    return lat + lon + time;
}

int main(void) {
    hal_time_init();
    hal_uart_init(UART_ID_TELEM, 115200);
    hal_gpio_init();

    hal_uart_write_nb(UART_ID_TELEM, (uint8_t *)"CYCLE BENCH START\n", 18);

    static volatile float vq[4] = {1.0f, 0.0f, 0.0f, 0.0f};
    static volatile float vgyro[3] = {0.01f, -0.02f, 0.005f};
    static volatile float vacc[3] = {0.05f, -0.03f, 0.998f};
    static volatile float va[9], vb[9], vc[9];
    static volatile uint8_t vframe[64];
    static volatile char vnmea[96] =
        "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";

    for (int i = 0; i < 9; i++) {
        va[i] = 0.1f * (float)(i + 1);
        vb[i] = 0.2f * (float)(9 - i);
    }

    /* Scalar float ops (N=1000, table-driven normal-range operands) */
    bench_copy(1000u);
    BENCH_BINOP("float_add", +, 1000u);
    BENCH_BINOP("float_sub", -, 1000u);
    BENCH_BINOP("float_mul", *, 1000u);
    BENCH_BINOP("float_div", /, 1000u);

    /* libm (N=200; soft-float is slow) */
    bench_unary_pos("sqrtf", 200u, sqrtf);
    bench_unary("sinf", 200u, sinf);
    bench_unary("cosf", 200u, cosf);
    {
        uint64_t t0 = hal_cycles();
        for (uint32_t i = 0u; i < 200u; i++) {
            fsink = atan2f(ftab[i & 7u], ftab[(i + 3u) & 7u]);
        }
        bench_report("atan2f", 200u, hal_cycles() - t0);
    }

    /* Mahony AHRS update (N=200) */
    {
        uint64_t t0 = hal_cycles();
        for (uint32_t i = 0; i < 200u; i++) {
            mahony_update(vq, vgyro, vacc);
        }
        bench_report("mahony_update", 200, hal_cycles() - t0);
    }

    /* 3x3 matmul (N=200) */
    {
        uint64_t t0 = hal_cycles();
        for (uint32_t i = 0; i < 200u; i++) {
            matmul3(vc, va, vb);
        }
        bench_report("matmul3x3", 200, hal_cycles() - t0);
    }

    /* MAVLink pack + parse/verify (N=200) */
    {
        uint64_t t0 = hal_cycles();
        for (uint32_t i = 0; i < 200u; i++) {
            mav_pack_heartbeat(vframe, (uint8_t)i);
        }
        bench_report("mavlink_pack_hb", 200, hal_cycles() - t0);
    }
    {
        mav_pack_heartbeat(vframe, 42);
        uint64_t t0 = hal_cycles();
        uint32_t ok = 0;
        for (uint32_t i = 0; i < 200u; i++) {
            ok += mav_parse_verify(vframe, 21);
        }
        bench_report("mavlink_parse_verify", 200, hal_cycles() - t0);
        vsink_u32 = ok;
    }

    /* NMEA GGA parse (N=200) */
    {
        uint64_t t0 = hal_cycles();
        for (uint32_t i = 0; i < 200u; i++) {
            nmea_parse_gga(vnmea);
        }
        bench_report("nmea_parse_gga", 200, hal_cycles() - t0);
    }

    hal_uart_write_nb(UART_ID_TELEM, (uint8_t *)"CYCLE BENCH DONE\n", 17);
    hal_gpio_write(GPIO_LED_GREEN, true);
    while (1) {
    }
    return 0;
}
