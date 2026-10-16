# VEGA Pilot Firmware - Architecture & Implementation Guide

**Status:** Milestone 0 Complete, Patches 1-4 Complete  
**Target:** THEJAS32 / VEGA ET1031 (RV32IM, no FPU) on ARIES v3 board  
**Emulator:** Thejas32rs (Rust instruction-set simulator, RV32IMAC+A)

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

**check-ram Target:**
- Counts `.data` + `.bss` (+ `.text` if copy-to-RAM)
- Stack painting: fills stack with 0xDEADBEEF at init
- Reports high-water mark (minimum free stack) at runtime
- XIP vs copy-to-RAM recorded as open item in AUDIT.md

### Linker-Symbol Loop Audit (all loops over linker symbols)

Every loop bounded by a linker-defined address was audited for the
unaligned-bound failure mode (a `!=`/`bne` loop over a 4-byte step never
terminates if the bound is odd — this exact bug hung boot until `link.ld`
was fixed). Rules: symbols word-aligned in `link.ld`, loops use bounded
`<` / `bltu` / `bgeu` comparisons, never `!=` / `bne` on addresses.

| Loop | Location | Bound symbols | Bound check | Alignment |
|------|----------|---------------|-------------|-----------|
| BSS zero | `crt0.s` | `__bss_start` → `__bss_end` | `bgeu` skip + `bltu` loop | Both `ALIGN(4)` in `link.ld` |
| Stack paint (0xDEADBEEF fill) | `src/main.c` | `__bss_end` → `__stack_top` | `while (ptr < top)` | Both word-aligned (`__stack_top` = 0x80040000) |
| Stack HWM scan | `src/main.c::stack_high_water_mark()` | `__bss_end` → `__stack_top` | `while (ptr < top)` | Same symbols (paint check `!= 0xDEADBEEF` is a *value* test, not a bound — correct) |
| `.data` copy (RAM init) | N/A — none exists | `__data_start/end` defined but unused at runtime | N/A | Symbols kept `ALIGN(4)` for future XIP copy-to-RAM (open item) |
| `check-ram` map parse | `Makefile` awk | `__text/data/bss_start/end` | `>` arithmetic only; `==` matches symbol *names*, never address `!=` | N/A (build-time) |
| Emulator ELF BSS fill | `src/bus.rs::load_elf` | `ram_offset + memsz`, clamped to `ram.len()` | Slice ranges, `min()`-clamped | Host-side, no target symbols |

`.text` bounds (`__text_start/end`) are `ALIGN(4)`-guarded for the same
reason although no runtime loop uses them today (check-ram uses them for
size arithmetic only).

---

## 8. Build System

### Toolchains
```make
# RV32 target
RV_CC := riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32
RV_CFLAGS := -std=c11 -Wall -Wextra -Werror -ffunction-sections -fdata-sections -Os -g \
             -ffreestanding -nostdlib -fno-builtin

# Host tests
HOST_CC := gcc -m64
HOST_CFLAGS := -std=c11 -Wall -Wextra -Werror -ffunction-sections -fdata-sections -Os -g
```

### Mem/Str Implementations
- Provide `memcpy`, `memset`, `memcmp`, `strlen`, `strcpy`, `strncpy` in `src/libc/`
- Or use newlib-nano subset (`-specs=nano.specs` if available)
- No full libc

### MAVLink Vendoring
- Vendor `c_library_v2` in `third_party/mavlink/`
- Include with `-isystem third_party/mavlink/include` (not `-I`) to suppress warnings
- Use only `minimal` + `common` dialects

### Targets
```bash
make rv32        # Build firmware/build/rv32/firmware.elf
make host-test   # Build firmware/build/host/test_runner
make test        # Run host unit tests (5 passing)
make emu-test    # Run firmware in emulator (100K cycles)
make check-ram   # Verify RAM usage + stack high-water
make clean       # Remove build/
```

### Linker Flags
- `--gc-sections` confirmed in `LDFLAGS`
- Explicit `libgcc.a` for soft-float

---

## 9. How to Extend (Next Milestones)

### Milestone 1: Scheduler, Time, Profiling

#### M1 Design Decisions — **Patches 1-4 COMPLETE**

**Emulator Patches (Patches 1-4 — DONE):**
*These patches have been implemented and merged. Firmware tasks can now be validated.*

| Priority | Emulator Patch | Status | Firmware Impact |
|----------|----------------|--------|-----------------|
| 1 | **3 UART instances** with separate Unix sockets (`/tmp/thejas_uart0/1/2.sock`) | ✅ Done | Enables independent `task_gnss_uart` (UART1) and `task_rc_uart` (UART2) validation |
| 2 | **I2C IPC protocol: command-only writes + multi-byte burst reads** (MS5611 read-3, HMC5883L read-6) | ✅ Done | Enables `task_baro` (MS5611 convert+read-3) and `task_mag` (HMC5883L read-6) |
| 3 | **Writable file-backed QSPI flash** (persist across runs) | ✅ Done | Enables M3 `PARAM_VALUE` persistence, logger flash writes |
| 4 | **`--strict-mem` flag**: access fault on unmapped loads/stores; PC=0 = fault (not clean exit) | ✅ Done | Hardens firmware against memory errors |
| 5 | **Test-finished register** that sets emulator exit code | ✅ Done | Clean `make emu-test` integration |
| 6 | **Baud-accurate UART TX model** (simulate bit timing) | Pending | Improves UART driver fidelity |
| 7 | **ADC clamp to 4095** (12-bit) | Pending | Prevents firmware ADC overflow |
| 8 | **`mcycleh` at 0xB80** (keep 0xC80 as user shadow) | Pending | Corrects CSR mapping |

*Current `hal_emu/i2c.c` has been updated for the NEW protocol (multi-byte reads).*

**Firmware Task Splitting (from single 1kHz "state estimate"):**
| Task | Rate | Period | Purpose |
|------|------|--------|---------|
| IMU (SPI) | 1000 Hz | 1000 µs | Burst read → raw IMU data |
| Baro (I2C) | 100 Hz | 10000 µs | Non-blocking convert/wait/read state machine |
| Mag (I2C) | 75 Hz | 13333 µs | Non-blocking read |
| GNSS UART drain | 100 Hz | 10000 µs | Non-blocking byte drain → NMEA parser |
| RC UART drain | 100 Hz | 10000 µs | Non-blocking byte drain → SBUS/CRSF parser |
| ADC | 100 Hz | 10000 µs | Non-blocking convert/read |

**Rule:** No task may block on I/O. All HAL calls are non-blocking or state-machine based.

**check-ram Enhancement:**
- Count `.data` + `.bss` (+ `.text` if linked into RAM for copy-to-RAM)
- Stack painting at init (fill stack with 0xDEADBEEF pattern)
- Report high-water mark (lowest stack address touched) at runtime — **IMPLEMENTED** via `stack_high_water_mark()` in `src/main.c`, reported in `vehicle_state_t.stack_hwm_bytes`
- Record XIP vs copy-to-RAM as open item in AUDIT.md

**Time Base:**
- `hal_time_us()` from CLINT `mtime` via `MTIME_HZ` in `config.h`
- Emulator: `MTIME_HZ = 100000000` (100 MHz nominal)
- Real hardware: set per datasheet (typically 32.768 kHz or 1 MHz)
- Use `mtimecmp` for scheduler tick (not Timer peripheral)
- Profiler output labeled **"instructions retired"** (emulator retires ~1 instruction per tick; real HW has pipeline stalls/cache effects — apply a 1.5-2x margin as an **UNMEASURED ASSUMPTION** until remeasured on hardware)

**Flash/Logger:**
- Erase and program as **chunked yielding state machines** (cooperative, no blocking)
- Each flash op yields to scheduler; logger flush is a state machine

**Toolchain:**
- `-ffreestanding` plus `mem*`/`str*` implementations (or newlib-nano subset)
- Vendor MAVLink with `-isystem` (not `-I`) to suppress warnings
- Confirm `--gc-sections` in `LDFLAGS` (already done)

**state.h Updates:**
- Document **single writer per field** (owner module)
- Add: `landed` flag, `current_mode`, `failsafe_status`, `timestamp`, `stack_hwm_bytes`
- Add `STATE_IFACE_VERSION` (compile-time version for integration checks)
- **Do not freeze until human signs off**

---

### Milestone 1: Scheduler, Time, Profiling (Implementation)
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
| `include/config.h` | **All tunables** - change rates, limits, defaults here (includes `MTIME_HZ`) |
| `include/state.h` | **Core interfaces** - single writer per field, `STATE_IFACE_VERSION`, not frozen |
| `include/hal_vega/regs.h` | **Register addresses** - single source of truth |
| `src/main.c` | Task registration + scheduler loop |
| `src/sched.c` | Scheduler implementation + profiling |
| `src/ctrl_stub/ctrl_stub.c` | Placeholder controller (replace this) |
| `src/hal_emu/*.c` | Emulator HAL implementations |
| `docs/AUDIT.md` | Full emulator analysis & differences |
| `firmware/Makefile` | Build system |

---

## 15. state.h Ownership & Versioning

### Single Writer Per Field
| Field | Writer | Readers |
|-------|--------|---------|
| `roll, pitch, yaw` | Attitude estimator (human) | Controller, MAVLink, logger |
| `rate_x, rate_y, rate_z` | IMU task | Attitude estimator, controller |
| `alt_m, climb_mps` | Altitude estimator (human) | Controller, MAVLink, logger |
| `lat_e7, lon_e7, vn, ve` | Position estimator (human) | Controller, MAVLink, logger |
| `att_valid, alt_valid, pos_valid` | Respective estimators | Mode manager, failsafe |
| `batt_mv, batt_ma, batt_pct` | ADC task | Failsafe, MAVLink |
| `gnss_fix, gnss_sats, gnss_last_ok_ms` | GNSS UART task | Mode manager, failsafe |
| `rc_link_ok, rc_last_ms` | RC UART task | Mode manager, failsafe |
| `gcs_last_heartbeat_ms` | MAVLink RX task | Failsafe |
| `armed` | Mode manager (arming logic) | All |
| `landed` | Land detector (human) | Mode manager, failsafe |
| `current_mode` | Mode manager | MAVLink, controller |
| `failsafe_status` | Failsafe manager | Mode manager, MAVLink |
| `home` | Mode manager (on arm) | Mission, RTL |
| `timestamp` | Scheduler (per cycle) | All (for latency measurement) |

### `STATE_IFACE_VERSION`
```c
#define STATE_IFACE_VERSION 0x00010000  // Major.Minor.Patch
```
Checked at link time via `static_assert` in modules that include `state.h`. **Do not freeze until human signs off.**

---

*Generated 2026-09-28 for Milestone 0 completion, updated with M1 design decisions*