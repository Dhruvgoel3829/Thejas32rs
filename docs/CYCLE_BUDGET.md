# VEGA Pilot Firmware: Cycle Budget

**Date:** 2026-10-16
**Method:** `firmware/programs/cycle_bench.c` (`make bench`) on Thejas32rs emulator.
All numbers are **instructions retired** (CSR `mcycle` deltas), NOT wall-clock
cycles — the emulator retires ~1 instruction per tick and models no pipeline,
caches, or memory stalls.

> **UNMEASURED ASSUMPTION:** real THEJAS32 hardware will be slower per
> instruction retired due to pipeline hazards, flash wait states, and
> peripheral bus latency. Apply a **1.5–2x margin** to every number below
> until remeasured on real hardware (Milestone 8). This factor has NOT been
> measured anywhere; it is a planning placeholder.

## Methodology

- Production flags: `-Os -march=rv32imac_zicsr -mabi=ilp32 -ffreestanding
  -nostdlib -fno-builtin`, same as firmware.
- Operands live in `volatile` tables of normal-range values, cycled per
  iteration, so nothing constant-folds and values never saturate to 0/inf
  (an earlier revision measured libm fast paths by accident; fixed).
- Scalar op cost = (op loop − copy-baseline loop) / N, where the baseline is
  a volatile load+store loop. Reported per-iter numbers INCLUDE loop
  overhead (~7 retired); marginal op cost ≈ reported − 7.
- Single run; repeated workloads reproduced identically across runs.

## Scalar float ops (per op, N=1000)

| Op | Retired/iter | Marginal over copy loop |
|----|------|-------------------------|
| copy baseline (load+store) | 7.01 | — |
| `float` add (`__addsf3`) | 82.50 | ~75.5 |
| `float` sub (`__subsf3`) | 92.63 | ~85.6 |
| `float` mul (`__mulsf3`) | 101.00 | ~94.0 |
| `float` div (`__divsf3`) | 101.63 | ~94.6 |

All via libgcc soft-float calls (no FPU). Note div ≈ mul on this libgcc
version; treat as measured, not as a general claim.

## libm (per call, N=200)

| Function | Retired/call |
|----------|------|
| `sqrtf` | 541.43 |
| `sinf` | 1915.93 |
| `cosf` | 2004.06 |
| `atan2f` | 2459.17 |

Linked from toolchain `libm.a` (`-lm`, `--start-group libgcc libm
--end-group`, plus an `int __errno` stub since freestanding has no libc
errno). **Code-size cost:** `cycle_bench.elf` `.text` is ~17 KB vs ~4.3 KB
for the base firmware — most of the delta is libm (`sinf`/`cosf`/`atan2f`
range reduction + polynomials). Prefer cheap approximations or LUTs in hot
paths; reserve full libm for init/calibration.

## Composite workloads (per invocation, N=200)

| Workload | Retired | Notes |
|----------|---------|-------|
| Mahony AHRS update (IMU-only, 2× `sqrtf`) | 7259.93 | Canonical form, Kp=1.0, dt=1 ms. At 1 kHz ≈ 7.3 M retired/s ≈ 7.3% of nominal 100 MIPS budget (before HW margin). |
| 3×3 matmul (27 mul + 18 add) | 4312.05 | ≈ 96/element-op incl. libgcc call overhead |
| MAVLink v2 pack (HEARTBEAT, 21 B frame + CRC-16/MCRF4XX + extra 50) | 378.04 | Hand-rolled minimal framing for benchmarking; full `c_library_v2` vendored in M3 |
| MAVLink v2 parse + CRC verify | 370.07 | Same note as pack |
| NMEA GGA parse (time/lat/lon to integers, manual digits, no `strtod`) | 584.08 | Representative of the GNSS UART task's parse cost |

## Scheduling implications (nominal 100 MHz, pre-margin)

- 1 kHz control loop budget: 100,000 retired/period. A Mahony update (7,260)
  consumes ~7.3% of one control slice; IMU SPI burst + mixer + PID must fit
  in the remainder.
- Baro state machine (100 Hz): I2C byte loops are single-digit retired per
  byte; conversion waits are wall-time, not CPU.
- MAVLink TX at 10 Hz with ~5 messages: ~5 × 378 ≈ 1,900 retired — negligible
  vs. the 10 ms (1 M retired) telemetry slice. UART TX *wall time* (baud
  timing), not CPU, is the binding constraint — hence ring buffers (M1).
- NMEA parse at 100 Hz: 584 retired — negligible; UART RX drain cost is noise.

## Reproducing

```bash
cd firmware
make bench
../target/release/Thejas32rs build/rv32/cycle_bench.elf --max-cycles 200000000
# BENCH lines print on UART0 (console stdout)
```

---

## Loop Budget Table (per-task worst case × rate)

Nominal budget: `MTIME_HZ` = 100 MHz → **100 M instructions-retired/s at 1x**.
Each row: worst-case retired per run × rate = retired/s. Tags say whether
the figure is MEASURED (this doc) or an ESTIMATE (engineering judgment,
to be replaced by measurement as modules land).

| Task | Rate | Worst-case retired/run | Load/s | Source |
|------|------|------------------------|--------|--------|
| control (stub + 4× PWM) | 1000 Hz | 500 | 0.50 M | ESTIMATE |
| imu (SPI 14 B burst + scale) | 1000 Hz | 800 | 0.80 M | ESTIMATE |
| baro (SM step + I2C seq, amortized) | 100 Hz | 1500 | 0.15 M | ESTIMATE |
| mag (I2C 6 B + scale) | 75 Hz | 500 | 0.04 M | ESTIMATE |
| gnss_uart (drain ≤64 B + GGA parse) | 100 Hz | 1600 | 0.16 M | MIXED (parse 584 MEASURED) |
| rc_uart (drain ≤64 B + SBUS parse) | 100 Hz | 1400 | 0.14 M | ESTIMATE |
| adc (3 ch + scale incl. div) | 100 Hz | 450 | 0.05 M | ESTIMATE |
| mode_fs (flags + timestamp) | 50 Hz | 200 | 0.01 M | PLACEHOLDER (M2) |
| mav_rx (drain + parse) | 100 Hz | 1000 | 0.10 M | ESTIMATE (parse 370 MEASURED) |
| mav_tx (5 msgs × pack) | 10 Hz | 1900 | 0.02 M | ESTIMATE (pack 378 MEASURED) |
| logger (HWM @0.1 Hz amortized + print) | 10 Hz | 1000 | 0.01 M | ESTIMATE (HWM scan 285k/run throttled ×0.01) |
| estimator (Mahony proxy) | 1000 Hz | 7260 | 7.26 M | MEASURED (human's estimator replaces this) |
| **Total** | | | **≈ 9.24 M/s** | |

| Margin | Budget | Load | % of budget | Pass (<50% @2x) |
|--------|--------|------|-------------|-----------------|
| 1x (nominal) | 100 M/s | 9.24 M/s | 9.2% | — |
| 2x (HW margin) | 50 M/s effective | 9.24 M/s | **18.5%** | **PASS** |
| 3x (pessimistic) | 33.3 M/s effective | 9.24 M/s | 27.7% | info only |

Headroom at 2x is dominated by the estimator proxy (Mahony 7.26 M/s =
78% of total load). If the human's estimator exceeds ~15 M/s at 1 kHz,
revisit: lower estimator rate, fixed-point, or LUT trig.

## I/O-Wait Analytic (wall time, NOT retired instructions)

The emulator does not model peripheral bus timing, so I/O wait is analytic
from **assumed** bus speeds (state assumptions explicitly):

- SPI IMU burst @ assumed 8 MHz: 14 B × 8 b / 8 MHz ≈ **14 µs** per 1 kHz sample (1.4% of period).
- MS5611 conversion @ datasheet OSR4096: **9 ms wall wait** (non-blocking state machine; zero CPU while waiting) + 3 B read @ assumed 100 kHz I2C ≈ 0.3 ms.
- HMC5883L 6 B read @ assumed 100 kHz I2C ≈ **0.6 ms** per 13.3 ms period (4.5%).
- UART TX @ 115200 baud: **~87 µs/byte**; a 100 B telemetry burst costs ~8.7 ms wall — this is why TX uses ring buffers, not why CPU is loaded.

If real bus speeds differ (e.g. 400 kHz I2C fast-mode, 20 MHz SPI), scale
linearly and update this section.

## Measurement specifics

- NMEA sentence under test (65 chars):
  `$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47`
  Cost scales ~linearly with sentence length; 65 chars is representative.
- MAVLink figures cover **HEARTBEAT (msgid 0)**: 9 B payload, 21 B frame
  (pack 378.04 / parse+verify 370.07 per invocation).
