# VEGA Pilot Firmware - Architecture & Implementation Guide

**Status:** Milestone 0 Complete (Audit & Scaffolding)  
**Target:** THEJAS32 / VEGA ET1031 (RV32IM, no FPU) on ARIES v3 board  
**Emulator:** Thejas32rs (Rust, cycle-accurate RV32IMAC+A)

---

## 1. Project Overview

This firmware provides the **bare-metal flight-software infrastructure** for the NIDAR 2026-27 Mission 3 "VEGA Pilot" competition. It builds everything *around* the flight control loops (attitude, altitude, position) which are being overhauled separately by the human engineer.

### In Scope (Agent Builds)
- Cooperative fixed-rate scheduler and time base
- HAL interface headers and emulator-backed implementations
- Flight **mode manager** (state machine)
- **Failsafe manager** (state machine)
- **Mission executor** (waypoint sequencing)
- **MAVLink layer**: telemetry, parameters, mission protocol, commands
- Logging (flight data, sensor inputs, commands, motor outputs)
- Host-side unit tests and emulator integration tests
- UART-socket-to-TCP bridge for Mission Planner / pymavlink
- Cycle-count profiling harness

### Out of Scope (Human Overhauls)
- Attitude/altitude/position estimation
- Rate/angle/position/altitude control loops
- Motor mixer tuning

---

## 2. Architecture

### High-Level Data Flow

```
                  +--------------------------------------------+
                  |                  main.c                    |
                  |     scheduler (fixed-rate task table)      |
                  +--------------------------------------------+
    +---------+  +---------+  +------------+  +----------+  +----------+
    | drivers |->|  state  |->|    mode    |->| setpoint |->| control  |
    | (HAL)   |  |  (bus)  |  |  manager   |  |  output  |  | (STUB /  |
    +---------+  +---------+  +-----+------+  +----------+  |  human)  |
                      ^             |                       +----------+
               +------+------+  +---+--------+
               |  failsafe   |->|  mission   |
               |  manager    |  |  executor  |
               +------+------+  +---+--------+
                      ^             ^
                      +------+------+
                          MAVLink
                     (telemetry, params,
                      mission, commands)
                          + logging
```

### Key Design Rule: Portable Core
All logic modules (mode manager, failsafe manager, mission executor, MAVLink handlers, parameter store, logger formatting) must be **pure C with no hardware access**, taking inputs through structs and function pointers. They must compile and run on:
- **Host** (`gcc`, x86-64) for unit tests
- **RV32** for target

Only `hal/` touches hardware registers.

---

## 3. Repository Layout

```
firmware/
├── Makefile              # Builds RV32 ELF + host test target
├── link.ld               # 256 KB RAM linker script
├── crt0.s                # Startup code (stack at 0x8004_0000)
├── include/
│   ├── config.h          # All tunables and limits (single source of truth)
│   ├── types.h           # Common types (status_t, time_us_t, cycles_t, etc.)
│   ├── sched.h           # Scheduler API (sched_task_t, sched_run)
│   ├── state.h           # Core interfaces: vehicle_state_t, setpoints_t, motor_out_t, ctrl_interface_t
│   ├── ctrl_stub.h       # Placeholder controller interface
│   ├── hal/              # HAL INTERFACE HEADERS ONLY (no implementation)
│   │   ├── imu.h         # SPI IMU (MPU-6000/ICM-20602)
│   │   ├── baro.h        # I2C Barometer (MS5611)
│   │   ├── mag.h         # I2C Compass (HMC5883L)
│   │   ├── gnss.h        # UART GNSS (NMEA parser in portable module)
│   │   ├── adc.h         # Battery voltage, current, RSSI
│   │   ├── pwm.h         # ESC outputs (4-8 channels)
│   │   ├── rc.h          # RC receiver (SBUS/CRSF/PPM parser in portable module)
│   │   ├── uart.h        # Non-blocking UART with ring buffers
│   │   ├── gpio.h        # LED, buzzer, safety switch
│   │   ├── flash.h       # QSPI flash for params/logs
│   │   └── time.h        # time_us(), time_ms(), cycles(), delays
│   ├── hal_vega/regs.h   # SINGLE SOURCE OF TRUTH for register addresses
│   └── hal/time_host.h   # Host test time mock helpers
├── src/
│   ├── main.c            # Scheduler loop + task registration
│   ├── sched.c           # Cooperative fixed-rate scheduler
│   ├── state/state.c     # Vehicle state + sensor update stubs
│   ├── ctrl_stub/ctrl_stub.c  # Placeholder controller (to be replaced by human)
│   ├── hal_emu/          # EMULATOR-BACKED HAL IMPLEMENTATIONS
│   │   ├── time.c        # CLINT mtime + CSR mcycle (hi/lo/hi safe read)
│   │   ├── uart.c        # 16550-style UART (blocking TX, non-blocking RX)
│   │   ├── gpio.c        # GPIO via MMIO
│   │   ├── spi.c         # IMU via SPI (burst read, WHO_AM_I verify)
│   │   ├── i2c.c         # MS5611 + HMC5883L via I2C
│   │   ├── adc.c         # Battery, current, RSSI via ADC
│   │   ├── pwm.c         # ESC outputs via PWM (normalized 0-1000)
│   │   ├── gnss.c        # NMEA parser stub (uses UART_ID_GNSS)
│   │   └── rc.c          # SBUS/CRSF parser stub (uses UART_ID_RC)
│   └── hal_vega/         # PLACEHOLDER for real hardware HAL (empty)
├── tests/host/test_main.c  # 5 unit tests (scheduler, state, ctrl, setpoints, params)
└── tools/                # For uart_tcp_bridge.py (M3+)
```

---

## 4. Core Interfaces (Fixed Early - Human Depends on These)

### `vehicle_state_t` (in `include/state.h`)
```c
typedef struct {
    // Estimated state (filled by human's estimators; stub fills from raw sensors)
    float roll, pitch, yaw;            // rad
    float rate_x, rate_y, rate_z;      // rad/s
    float alt_m, climb_mps;            // relative to home/launch
    int32_t lat_e7, lon_e7;
    float vn, ve;                      // m/s
    bool  att_valid, alt_valid, pos_valid;

    // Health / inputs
    uint16_t batt_mv, batt_ma; uint8_t batt_pct;
    uint8_t  gnss_fix, gnss_sats; uint32_t gnss_last_ok_ms;
    bool  rc_link_ok;  uint32_t rc_last_ms;
    uint32_t gcs_last_heartbeat_ms;
    bool  armed;
    home_t home;                       // set on arm
} vehicle_state_t;
```

### `setpoints_t` (in `include/state.h`)
```c
typedef enum { CTRL_NONE, CTRL_STABILIZE, CTRL_ALT_HOLD, CTRL_POS_HOLD,
               CTRL_WAYPOINT, CTRL_LAND, CTRL_DISARMED } ctrl_request_t;

typedef struct {
    ctrl_request_t request;
    // Attitude/throttle targets (STABILIZE / manual assist)
    float roll_t, pitch_t, yaw_rate_t, throttle;
    // Navigation targets (ALT_HOLD / POS_HOLD / WAYPOINT / LAND)
    int32_t tgt_lat_e7, tgt_lon_e7; float tgt_alt_m;
    float max_speed_mps, climb_rate_mps, descend_rate_mps;
} setpoints_t;
```

### `ctrl_interface_t` (in `include/state.h`) - **THE SEAM**
```c
typedef struct {
    void (*init)(void);
    void (*update)(const vehicle_state_t*, const setpoints_t*, motor_out_t*);
    void (*reset)(void);
} ctrl_interface_t;
```
**Human replaces `ctrl_stub` by providing their own implementation of this interface.** See `src/ctrl_stub/ctrl_stub.c` for the placeholder.

---

## 5. Scheduler & Time Base

### Cooperative Fixed-Rate Scheduler (`src/sched.c`)
- Static task table: `{fn, period_us, last_run, max_cycles_seen, overruns, name}`
- No RTOS, no preemption, no dynamic allocation
- Time base: CLINT `mtime` (64-bit) + CSR `mcycle` (64-bit)
- **Safe 64-bit reads on RV32** via hi/lo/hi re-read loop (see `hal_emu/time.c`)

### Task Rates (from `config.h`)
| Task | Rate | Period | Purpose |
|------|------|--------|---------|
| Control loop | 1000 Hz | 1000 µs | `ctrl_interface->update()` |
| State estimate | 1000 Hz | 1000 µs | Read sensors → `vehicle_state_t` |
| Mode + Failsafe | 50 Hz | 20000 µs | Mode transitions, failsafe checks |
| MAVLink RX | 100 Hz | 10000 µs | Drain UART FIFO, parse frames |
| MAVLink TX | 10 Hz | 100000 µs | Periodic telemetry (HEARTBEAT 1 Hz, etc.) |
| Logger | 10 Hz | 100000 µs | Flush log buffer to QSPI |

### Profiling (M1 Requirement)
- Per-task cycle counting via `mcycle`
- Track min/avg/max cycles and overrun counts
- Expose via debug UART command and log

---

## 6. HAL Architecture

### Interface Headers (`include/hal/*.h`)
- Pure C, no implementation
- Each returns `status_t` (or `bool` for simple init/read)
- None block for long
- Emulator implementations in `src/hal_emu/`
- Real hardware implementations will go in `src/hal_vega/`

### Register Addresses
**All real register addresses live ONLY in `include/hal_vega/regs.h`**. Currently populated from emulator memory map - **must be verified against C-DAC SDK headers** when real board arrives.

```c
// Example from hal_vega/regs.h
#define CLINT_BASE        0x02000000
#define CLINT_MTIME       (CLINT_BASE + 0xBFF8)
#define CLINT_MTIMEH      (CLINT_BASE + 0xBFFC)
#define UART_BASE         0x10000000
#define GPIO_BASE         0x10010000
#define SPI_BASE          0x10030000
#define I2C_BASE          0x10050000
#define ADC_BASE          0x10080000
#define PWM_BASE          0x10090000
// ... etc
```

### Emulator Backend Details (`src/hal_emu/`)
- Uses existing Unix sockets (`/tmp/thejas_spi.sock`, `_gpio`, `_i2c`, `_uart`, `_adc`, `_pwm`)
- Wire formats documented in `docs/AUDIT.md` and `MEMORY.md`
- **Do not modify emulator core** unless a bug blocks progress

---

## 7. Memory Budget

| Region | Size | Notes |
|--------|------|-------|
| **SRAM Budget** | **256 KB** | Real hardware (emulator has 512 KB) |
| Stack Reserve | 32 KB | `__stack_top = 0x8004_0000` |
| Available for .bss/.data | ~224 KB | Static buffers only |
| Flash (QSPI XIP) | 16 MB | At `0x4000_0000` |

**Rules:**
- No `malloc`/`free` - all memory static
- Fixed-size buffers with compile-time bounds in `config.h`
- Linker map checked: build fails if RAM > `SRAM_BUDGET_BYTES`

---

## 8. Build System

### Toolchains
```make
# RV32 target
RV_CC := riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32
RV_CFLAGS := -std=c11 -Wall -Wextra -Werror -ffunction-sections -fdata-sections -Os -g -nostdlib -fno-builtin

# Host tests
HOST_CC := gcc -m64
HOST_CFLAGS := -std=c11 -Wall -Wextra -Werror -ffunction-sections -fdata-sections -Os -g
```

### Targets
```bash
make rv32        # Build firmware/build/rv32/firmware.elf
make host-test   # Build firmware/build/host/test_runner
make test        # Run host unit tests (5 passing)
make emu-test    # Run firmware in emulator (100K cycles)
make check-ram   # Verify BSS < 229376 bytes (256KB - 32KB stack)
make clean       # Remove build/
```

### Soft-Float
- No FPU on target → all `float` ops via libgcc (`__mulsf3`, `__addsf3`, etc.)
- Link with `libgcc.a` explicitly in Makefile
- Never use `double`

---

## 9. How to Extend (Next Milestones)

### Milestone 1: Scheduler, Time, Profiling
1. **UART ring buffers** in `hal_emu/uart.c` - non-blocking TX with backpressure
2. **Cycle profiling** in `sched.c` - min/avg/max per task, overrun detection
3. **Safe 64-bit time** - already in `hal_emu/time.c` (hi/lo/hi loop)
4. **Microbenchmarks** - create `docs/CYCLE_BUDGET.md` with float op cycles
5. **Demo** - add dummy tasks at different rates, run on emulator

### Milestone 2: Mode & Failsafe Managers (Host-Tested)
- Implement `mode/mode_manager.c` per Section 7.1 (ArduCopter custom_mode mapping)
- Implement `failsafe/failsafe.c` per Section 7.3 (latching, escalation)
- Host tests for every transition, arming refusal, failsafe row

### Milestone 3: MAVLink Core
- HEARTBEAT, telemetry streams, REQUEST_DATA_STREAM
- Parameters with QSPI flash persistence (CRC)
- Commands, STATUSTEXT
- Build `tools/uart_tcp_bridge.py`
- Verify with pymavlink: connect, read params, change, power-cycle, persist

### Milestone 4: Mission Protocol & Executor
- Mission upload/download state machines with timeouts
- Waypoint sequencing (`MISSION_ITEM_REACHED`, `MISSION_CURRENT`)
- RTL/LAND sequences with phases
- pymavlink round-trip test

### Milestone 5: Mission Planner Integration
- Manual demo documented in `docs/DEMO.md`
- Scripted failsafe scenarios: RC loss, GCS loss, low/crit battery, GNSS loss

### Milestone 6: Logging
- Ring-buffered binary log to QSPI
- Python decode tool → CSV

### Milestone 7: Controller Seam & Handoff
- Freeze interfaces in `include/state.h`
- Document replacement procedure in `docs/INTEGRATION.md`
- Compile-time switch for human's controllers

### Milestone 8: Hardware Readiness (When Board Available)
- Real `hal_vega/` implementation against C-DAC SDK
- Diff register maps against emulator
- Re-run cycle measurements on hardware
- Re-verify SRAM budget

---

## 10. Running the Firmware

### Build & Test (Host)
```bash
cd firmware
make test
# Output:
# === VEGA Pilot Firmware Host Tests ===
# test_sched_basic: PASS
# test_state_init: PASS
# test_ctrl_stub: PASS
# test_setpoints: PASS
# test_param_bounds: PASS
# === All Tests Passed ===
```

### Build & Run (Emulator)
```bash
cd firmware
make rv32
# Runs emulator for 100K cycles
./target/release/Thejas32rs build/rv32/firmware.elf --max-cycles 100000

# With IPC (for Gazebo SITL)
./target/release/Thejas32rs build/rv32/firmware.elf --ipc --max-cycles 100000
```

### GDB Debugging
```bash
# Terminal 1
./target/release/Thejas32rs firmware/build/rv32/firmware.elf --gdb-port 1234

# Terminal 2
riscv64-unknown-elf-gdb firmware/build/rv32/firmware.elf
(gdb) target remote :1234
(gdb) break main
(gdb) continue
```

---

## 11. Coding Rules (Enforced)

- C11, `-Wall -Wextra -Werror`, `-ffunction-sections -fdata-sections -Wl,--gc-sections`, `-Os`
- Float only. Avoid `<math.h>` in hot paths. No `double`.
- No dynamic allocation, no recursion, no unbounded loops, no VLAs
- Every module states worst-case stack use and static RAM in comment
- Build prints RAM/flash usage from linker map, **fails** if > `SRAM_BUDGET_BYTES`
- Every magic number in `config.h` or parameter table
- All time comparisons use unsigned subtraction (wrap-safe)
- Small commits per milestone, short changelog entry

---

## 12. Things to Report Back (Don't Guess)

1. Emulator vs real THEJAS32/ARIES v3 mismatches
2. Emulator bugs blocking milestones
3. Conflicts between brief requirements and implementation choices
4. Parameter defaults needing flight-test tuning (→ `docs/TUNING.md`)
5. Extra data human's controller needs beyond Section 6 interfaces

---

## 13. Definition of Done (Agent's Part)

- [x] M0: Audit & scaffolding
- [ ] M1: Scheduler, time, profiling
- [ ] M2: Mode & failsafe managers (host-tested)
- [ ] M3: MAVLink core
- [ ] M4: Mission protocol & executor
- [ ] M5: Mission Planner integration & full failsafe scenarios
- [ ] M6: Logging
- [ ] M7: Controller seam & hand-off
- [ ] M8: Hardware readiness (when board available)

All host tests passing, all emulator scenarios scripted and passing, Mission Planner demo documented, RAM/flash budget in CI output, cycle budget document produced, controller seam proven with alternate implementation.

---

## 14. Key Files to Know

| File | Purpose |
|------|---------|
| `include/config.h` | **All tunables** - change rates, limits, defaults here |
| `include/state.h` | **Core interfaces** - human's controller plugs into `ctrl_interface_t` |
| `include/hal_vega/regs.h` | **Register addresses** - single source of truth |
| `src/main.c` | Task registration + scheduler loop |
| `src/sched.c` | Scheduler implementation |
| `src/ctrl_stub/ctrl_stub.c` | Placeholder controller (replace this) |
| `src/hal_emu/*.c` | Emulator HAL implementations |
| `docs/AUDIT.md` | Full emulator analysis & differences |
| `firmware/Makefile` | Build system |

---

*Generated 2026-09-28 for Milestone 0 completion*