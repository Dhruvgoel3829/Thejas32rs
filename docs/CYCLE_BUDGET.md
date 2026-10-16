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
