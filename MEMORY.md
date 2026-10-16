# Thejas32rs Project Memory

## Project Overview
Cycle-accurate **RV32IM** RISC-V SoC emulator in Rust implementing the custom **THEJAS32** processor with rich peripherals and GDB stub.

**Key Specs:**
- RV32IM + RV32C (compressed) + RV32A (atomic) instruction sets
- 16-bit compressed instruction decode via `c_decode.rs`
- Machine-mode only (priv=3), full CSR model with traps
- PMP (16 entries, TOR/NA4/NAPOT modes)
- 12 peripherals memory-mapped
- GDB stub over TCP with hardware breakpoints

## Current State (as of commit e86e32a)

### Completed Features
| Component | Status |
|-----------|--------|
| CPU Core (RV32IM) | ✅ Complete |
| RV32C Compressed Instructions | ✅ Complete |
| RV32A Atomic Extension | ✅ Complete (LR.W, SC.W, AMO*) |
| CLINT (64-bit timer + MSIP) | ✅ Complete |
| PLIC (32 sources, 8 priorities) | ✅ Complete |
| UART (16550-style, crossterm) | ✅ Complete |
| GPIO | ✅ Complete |
| Timer (prescaler, auto-reload) | ✅ Complete |
| SPI (loopback + virtual flash) | ✅ Complete |
| I2S | ✅ Complete |
| I2C (virtual 24C02 EEPROM) | ✅ Complete |
| DMA (16 bytes/cycle, inc modes) | ✅ Complete |
| WDT (reset + interrupt modes) | ✅ Complete |
| QSPI Flash (16MB XIP window) | ✅ Complete |
| PMP (16 entries) | ✅ Complete |
| GDB Stub (TCP, breakpoints, regs/mem) | ✅ Complete |
| ELF Loader (goblin) | ✅ Complete |
| Scheduler Test (preemptive RTOS) | ✅ Working |
| Flight Controller (SPI IMU + PID) | ✅ Working |
| **Unit Tests (115 tests)** | ✅ Complete |

### Memory Map
```
0x0000_0000 – 0x3FFF_FFFF  — Unmapped (returns 0)
0x4000_0000 – 0x40FF_FFFF  — QSPI Flash (16 MB XIP)
0x8000_0000 – 0x8007_FFFF  — SRAM (512 KB)
0x0200_0000 – 0x0200_FFFF  — CLINT
0x0C00_0000 – 0x0C3F_FFFF  — PLIC
0x1000_0000 – 0x1000_FFFF  — UART
0x1001_0000 – 0x1001_FFFF  — GPIO
0x1002_0000 – 0x1002_FFFF  — Timer
0x1003_0000 – 0x1003_FFFF  — SPI
0x1004_0000 – 0x1004_FFFF  — I2S
0x1005_0000 – 0x1005_FFFF  — I2C
0x1006_0000 – 0x1006_FFFF  — DMA
0x1007_0000 – 0x1007_FFFF  — WDT
```

### Test Programs (in programs/)
- `hello.c` - Basic UART hello world
- `compressed_test.c` - RV32C factorial test
- `atomic_test.c` - AMOADD, AMOSWAP, LR/SC CAS test
- `timer_test.c` - Timer interrupt test
- `timer_poll_test.c` - Timer auto-reload polling test
- `plic_test.c` - PLIC interrupt test
- `scheduler_test.c` + `scheduler_asm.s` - Preemptive scheduler with trap entry
- `flight_controller.c` - SPI IMU + PID control loop (simple rate PID)
- `feedback_controller.c` - Full cascade PID (angle→rate) + complementary filter
- `drone_test.c` - System test: IMU, motor sequence, hover ramp
- `drone_peripheral_test.c` - **Comprehensive test of all 8 Gazebo-exposed peripherals**
- `dma_test.c`, `i2c_test.c`, `wdt_test.c`, `pmp_test.c`, `mixer_test.c`, `imu_test.c`, `comprehensive_test.c`, `xip_test.c`, `peripherals_test.c`, `echo.c`, `shell.c`

### Build & Run
```bash
cargo build --release
./target/release/Thejas32rs <elf> [--trace] [--dump-regs] [--max-cycles N] [--gdb-port N] [--ipc]
```

### GDB Debugging
```bash
# Terminal 1
./target/release/Thejas32rs hello.elf --gdb-port 1234

# Terminal 2
riscv64-unknown-elf-gdb hello.elf
(gdb) target remote :1234
(gdb) break main
(gdb) continue
```

### IPC / SITL Mode
```bash
# Terminal 1: Gazebo
cd gazebo/build && gzserver ../worlds/iris.world

# Terminal 2: Python bridge (Gazebo UDP ↔ Unix sockets)
python3 tools/sitl_bridge.py

# Terminal 3: Emulator with IPC
./target/release/Thejas32rs programs/flight_controller.elf --ipc
```

## Known Gaps / Unimplemented

### CPU Instructions (src/cpu/mod.rs)
All standard RV32IM + RV32A + RV32C instructions implemented:
- RV32M: MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU
- RV32A: LR.W, SC.W, AMOSWAP, AMOADD, AMOXOR, AMOAND, AMOOR, AMOMIN, AMOMAX, AMOMINU, AMOMAXU
- RV32C: Full decompression

Reserved/unknown opcodes now trap with illegal instruction (cause 2) instead of panicking:
- LOAD funct3=3,6,7 (FLW - FPU, not in THEJAS32)
- STORE funct3=3-7 (FSW - FPU, not in THEJAS32)
- Reserved R/M type combinations
- Reserved AMO funct5 variants
- Unknown opcodes

### Recently Fixed (2026-09-27)
- ✅ **System funct12=0x121** - SFENCE.VMA (implemented as no-op, no virtual memory)
- ✅ **CSR funct3=0x4** - Reserved SYSTEM funct3 now traps with illegal instruction (cause 2)
- ✅ **ELF Loading Bug** - Fixed PT_LOAD segment handling for segments overlapping RAM (including BSS-only segments with filesz=0). All test programs now load correctly.
- ✅ **Unit Test Suite** - Added 115 comprehensive unit tests covering CPU decode, CSR/PMP, and all peripherals
- ✅ **Illegal Instruction Traps** - All unimplemented!() macros replaced with proper illegal instruction traps (cause 2, mtval = instruction). Unknown/reserved opcodes now trap gracefully instead of panicking.

### Missing Architecture Features
- RV32F/D (FPU) - not in scope (RV32IM only)
- Supervisor/User mode - Machine mode only
- Vector/Hypervisor extensions - not implemented

### Dead/Unused Code (Intentionally Retained)
- `RawModeGuard` in main.rs - Reserved for future raw terminal mode feature
- `*_SIZE` constants in peripheral modules - Documentation of peripheral address space sizes
- `CSR_PMPCFG1/2/3` in csr.rs - Defined for completeness (range CSR_PMPCFG0..=CSR_PMPCFG3 used)
- `Clint::is_software_interrupt_pending()` - API completeness
- `SocketBackend::wait_for_client()` / `recv()` - Reserved for future IPC features

### Testing
- **132 unit tests** passing (cargo test)
  - CPU decode: 22 tests (RV32I, M, A, C, System mnemonics)
  - CSR: 10 tests (read/write, MISA, cycle, PMP config/addr)
  - PMP: 18 tests (config/addr R/W, TOR/NA4/NAPOT, access checks)
  - Peripherals: 65 tests (CLINT, PLIC, Timer, GPIO, SPI, I2S, I2C, DMA, WDT)
  - ADC: 8 tests (register R/W, channel conversion, status flags)
  - PWM: 9 tests (register R/W, freq/duty/dead-time/phase, clamping)
- Integration testing via manual ELF execution (hello, compressed, atomic, timer, scheduler, flight controller, drone_peripheral_test, etc.)

## Project Structure
```
Thejas32rs/
├── Cargo.toml              # edition 2024, deps: goblin, crossterm, clap
├── .gdbinit                # GDB config
├── .gitignore              # Cargo artifacts + MEMORY.md + COMMANDS.md
├── src/
│   ├── main.rs             # CLI, execution loop, GDB integration, --ipc flag
│   ├── bus.rs              # Memory bus, ELF loader, peripheral mapping
│   ├── cpu/
│   │   ├── mod.rs          # CPU core (644 lines) - fetch/decode/execute
│   │   ├── decode.rs       # 32-bit instruction decode
│   │   ├── c_decode.rs     # 16-bit → 32-bit decompression (256 lines)
│   │   ├── csr.rs          # CSR file + PMP
│   │   ├── pmp.rs          # Physical Memory Protection
│   │   └── execute.rs      # Placeholder (empty)
│   ├── clint.rs            # Core Local Interruptor
│   ├── plic.rs             # Platform-Level Interrupt Controller
│   ├── uart.rs             # UART + crossterm polling thread (no external deps)
│   ├── gpio.rs             # GPIO with optional IPC (Unix socket)
│   ├── timer.rs            # Programmable timer
│   ├── spi.rs              # SPI controller with optional IPC (Unix socket)
│   ├── i2s.rs              # I²S audio
│   ├── i2c.rs              # I²C + virtual EEPROM
│   ├── dma.rs              # DMA engine
│   ├── wdt.rs              # Watchdog timer
│   ├── qspi.rs             # QSPI flash window
│   ├── gdb.rs              # GDB stub protocol
│   ├── ipc.rs              # Unix socket IPC (SocketBackend for external bridges)
│   └── gazebo_backend.rs   # Gazebo UDP backends (used by Python bridges only)
├── programs/
│   ├── MakeFile            # Builds test ELFs (needs riscv-none-elf-gcc)
│   ├── link.ld             # Linker script
│   ├── crt0.s              # C runtime startup
│   ├── comp_entry.s        # Compressed test entry
│   ├── scheduler_asm.s     # Trap entry + context switch
│   └── *.c / *.elf         # Test programs
├── gazebo/
│   ├── CMakeLists.txt      # Gazebo plugin build
│   ├── gazebo_thejas_sitl.cpp
│   └── worlds/iris.world   # Quadcopter model
└── tools/                  # Python SITL bridges (external to core)
```

## Key Implementation Details

### Execution Loop (main.rs:107-118)
```rust
loop {
    cpu.step(cli.trace);
    cycle_count += 1;
    if cpu.pc == 0 { break; }
    if cli.max_cycles > 0 && cycle_count >= cli.max_cycles { break; }
}
```

### Step Function (cpu/mod.rs)
1. Tick peripherals (WDT, UART, Timer, DMA, CLINT)
2. Check interrupts (CLINT MTIP bit 7, PLIC MEIP bit 11)
3. Fetch (16/32-bit via PMP check)
4. Decompress if compressed
5. Decode → Execute → Advance PC

### Interrupt Priority
- External (PLIC/MEIP) > Timer (CLINT/MTIP)
- Checked every cycle before fetch

### Trap Handling
- mepc ← PC, mcause ← cause, MPIE ← MIE, MIE ← 0
- PC ← mtvec (direct) or mtvec + cause×4 (vectored)

### PMP Check
- On every instruction fetch, load, store
- 16 entries, modes: OFF, TOR, NA4, NAPOT
- Lock bits prevent modification in M-mode

## Dependencies
| Crate | Version | Purpose |
|-------|---------|---------|
| goblin | 0.8 | ELF parsing |
| crossterm | 0.28 | Raw terminal I/O |
| clap | 4.5 | CLI parsing |

## Next Priority Tasks
All THEJAS32 (RV32IM + RV32C + RV32A) scope items complete:

✅ CPU Core (RV32IM)
✅ RV32C Compressed Instructions  
✅ RV32A Atomic Extension
✅ All 12 peripherals (CLINT, PLIC, UART, GPIO, Timer, SPI, I2S, I2C, DMA, WDT, QSPI, PMP)
✅ GDB Stub, ELF Loader
✅ Scheduler Test, Flight Controller
✅ 132 unit tests (115 core + 17 new)
✅ Illegal instruction traps (cause 2)
✅ 7/12 peripherals exposed to Gazebo SITL
✅ Comprehensive drone peripheral test program

No remaining tasks within THEJAS32 scope. Project is feature-complete for its defined ISA.

### Optional Enhancements (Outside Core Scope)
- CAN bridge for Gazebo (UAVCAN/ESC telemetry)
- Timer capture/compare Gazebo exposure
- Remaining peripheral Gazebo exposure (QSPI, DMA, WDT, I2S)
- Flight controller PID tuning with real Gazebo physics
- **NIDAR VEGAPilot Flight Controller Requirements** (from Mission Brief)

## NIDAR VEGAPilot Flight Controller Requirements (Mission Brief)

### Core Constraints
- **Processor**: ARIES v3 - VEGA Processor Board (sole processing platform)
- **No additional processors/MCUs** for any autopilot function
- **No Chinese-origin components** (complete supply chain restriction)
- **Only prescribed components** from organizer's list
- **Custom PCBs allowed** only for integration (no unlisted active components)

### Required Flight Modes
| Mode | Description |
|------|-------------|
| arm / disarm | Safe arming/disarming with safety checks |
| manual / assisted stabilized | Direct motor control with attitude stabilization |
| position hold | GNSS-based position hold |
| altitude hold | Barometer-based altitude hold |
| autonomous mission mode | Waypoint navigation |
| return-to-home / return-to-launch | RTL/RTL failsafe |

### Required Capabilities
| Category | Requirements |
|----------|--------------|
| **Sensor Processing** | IMU (accel/gyro), Barometer, Compass, GNSS |
| **Motor Control** | ESC outputs (4+ channels), PWM/DShot |
| **Attitude Control** | Stable hover, roll/pitch/yaw response |
| **Navigation** | Waypoint mission, position hold, RTL |
| **Communication** | MAVLink telemetry to GCS (Mission Planner) |
| **Failsafe** | Low battery, link loss, GNSS loss, RTL, emergency landing |
| **Logging** | Flight data, sensor inputs, motor outputs |

### Required Interfaces (per Mission Brief)
| Interface | Purpose | Peripheral |
|-----------|---------|------------|
| **SPI** | IMU (MPU-6000/ICM-20602) | ✅ Exposed |
| **I2C** | Baro (MS5611), Compass (HMC5883L), GPS | ✅ Exposed |
| **UART** | GNSS NMEA, Telemetry, RC receiver | ✅ Exposed |
| **ADC** | Battery voltage, Current sensor, RSSI | ✅ Exposed |
| **PWM** | ESC outputs (4-8 channels) | ✅ Exposed |
| **GPIO** | LED status, Buzzer, Safety switch | ✅ Exposed |

### Final Mission Demonstration Requirements
1. **Pre-flight**: Sensor checks, motor outputs, comms verification
2. **Arm/Disarm**: Safe arming sequence
3. **Takeoff**: Designated launch area
3. **Stable hover**: Specified duration
4. **Control response**: Roll, pitch, yaw, throttle
4. **Navigation**: Forward/back/left/right with stability
5. **Course navigation**: Gates/markers in sequence
5. **Stable hover at waypoints**: Designated points
7. **Controlled recovery**: Return to neutral inputs
8. **Landing**: Designated zone, safe landing
8. **Data logging**: Sensor inputs, commands, motor outputs

### Communication Requirements
- **Protocol**: MAVLink (ArduPilot/PX4 compatible)
- **Telemetry**: Location, altitude, mode, battery, mission status, failsafe, link status
- **GCS**: Mission Planner / Ground Control Station
- **Commands**: Mission upload, waypoints, flight modes, RTL/abort

### Safety & Failsafe (Mandatory)
- Low battery detection & action
- Command/control link loss → RTL
- GNSS signal loss → RTL/landing
- Unstable flight → Emergency landing
- Mission abort capability
- Return-to-home / Return-to-launch
- Emergency landing

### Supply Chain Compliance
- **Zero Chinese-origin components** (resistors to ICs)
- **Bill of Materials**: Make, model, manufacturer, country, supplier
- **Documentation**: Datasheets, invoices, supplier confirmations
- **Only prescribed components** from organizer list

### Development Platform
- **ARIES v3 - VEGA Processor Board** (2 boards provided)
- **VEGA ISA**: RISC-V (RV32IMAC compatible)
- **Open-source leverage**: ArduPilot, PX4, MAVLink allowed
- **Two boards**: Development + final integration

### ArduPilot VEGA Port Progress

| Task | Status | Notes |
|------|--------|-------|
| **hwdef.dat template** | ✅ Complete | All peripherals mapped, pins as TODO |
| **board_config.h template** | ✅ Complete | Clock, memory, peripheral config |
| **board.c template** | ✅ Complete | Peripheral init (SPI, I2C, UART, PWM, ADC, GPIO) |
| **ChibiOS VEGA port** | ⏳ Pending | Verify/port ChibiOS for VEGA |
| **ArduPilot build** | ⏳ Pending | ./waf configure --board=vega |
| **SITL testing** | ⏳ Pending | Test with emulator --ipc |
| **Hardware bring-up** | ⏳ Pending | Flash to real VEGA boards |

## IPC-Based External Integration (AI-in-the-Loop Drone Firmware Development)

### Architecture (IPC Layer - Clean Core Emulator)
```
┌─────────────────┐    Unix Sockets     ┌──────────────────┐
│  Thejas32rs     │ ◄──────────────────► │  Python Bridges  │
│  (Core Emulator)│                     │  (External)      │
│                 │  /tmp/thejas_spi.sock │                  │
│  SPI ←→ IMU     │  /tmp/thejas_gpio.sock│  sitl_bridge.py  │
│  GPIO ←→ Motors │                     │  motor_bridge.py   │
└─────────────────┘                     └────────┬─────────┘
                                                 │
                          UDP                    ▼
                    ┌─────────────────────────────────────┐
                    │         Gazebo Plugin (C++)         │
                    │  (libthejas_sitl.so + iris.world)   │
                    │  - Receives motor commands (port 9003)│
                    │  - Sends IMU data (port 9002)        │
                    └─────────────────────────────────────┘
```

### Core Emulator (No External Dependencies)
- **SPI** (`src/spi.rs`) - Uses `SocketBackend` via IPC flag, loopback mode when disabled
- **GPIO** (`src/gpio.rs`) - Uses `SocketBackend` via IPC flag, local only when disabled
- **UART** (`src/uart.rs`) - Pure crossterm terminal I/O (no external deps)
- **CLI** - `--ipc` flag enables Unix socket servers

### IPC Layer (`src/ipc.rs`)
| Component | Socket Path | Purpose |
|-----------|-------------|---------|
| `SocketBackend` (SPI) | `/tmp/thejas_spi.sock` | Full-duplex SPI exchange with external IMU |
| `SocketBackend` (GPIO) | `/tmp/thejas_gpio.sock` | Motor command output (u32 little-endian) |

### Python Bridges (`tools/`)
| Bridge | Purpose |
|--------|---------|
| `sitl_bridge.py` | Full pipeline: Gazebo `gz topic` CLI ↔ Unix sockets ↔ Emulator |
| `motor_bridge.py` | Monitor motor commands from `/tmp/thejas_gpio.sock` |
| `external_imu_proxy.py` | Synthetic IMU server for `/tmp/thejas_spi.sock` |

### Gazebo C++ Plugin (Separate Process)
| Component | File | Protocol |
|-----------|------|----------|
| Plugin | `gazebo/gazebo_thejas_sitl.cpp` | UDP 9002 (IMU out), 9003 (motors in) |
| World | `gazebo/worlds/iris.world` | Quadcopter with IMU sensor + 4 rotor joints |

### UDP Protocol (Gazebo ↔ Python Bridge)
| Port | Direction | Payload |
|------|-----------|---------|
| 9002 | Gazebo → Bridge | 7×double (56B): timestamp, ax,ay,az, gx,gy,gz |
| 9003 | Bridge → Gazebo | 4×float (16B): motor commands 0.0-1.0 (normalized from GPIO 0-255) |

### Usage (Three-Terminal Setup)
```bash
# Terminal 1: Start Gazebo with Iris world
cd gazebo/build && gzserver ../worlds/iris.world

# Terminal 2: Run Python bridge (connects Gazebo UDP ↔ Emulator Unix sockets)
python3 tools/sitl_bridge.py

# Terminal 3: Run emulator with IPC enabled
./target/release/Thejas32rs programs/flight_controller.elf --ipc
```

### Benefits of IPC Architecture
1. **Clean Core** - Emulator has zero Gazebo/physics dependencies
2. **Testability** - Run tests without Gazebo (`--ipc` not required)
3. **Flexibility** - Swap Python bridges for different simulators (PX4, ArduPilot, etc.)
4. **Debugging** - `motor_bridge.py` shows motor values in real-time
5. **Language Agnostic** - External bridges can be Python, C++, Rust, etc.

### Fixed Issues (2026-09-25)
- ✅ **System funct12=0x121** - SFENCE.VMA (implemented as no-op, no virtual memory)
- ✅ **CSR funct3=0x4** - Reserved SYSTEM funct3 now traps with illegal instruction (cause 2)
- ✅ **ELF Loading Bug** - Fixed PT_LOAD segment handling for segments overlapping RAM (including BSS-only segments with filesz=0). All test programs now load correctly.
- ✅ **IPC Architecture** - Core emulator uses Unix sockets, Gazebo handled by external Python bridges
- ✅ **Gazebo SITL Link** - Python bridge forwards motor commands (UDP 9003), Gazebo plugin applies forces/torques to base_link for actual flight physics
- ✅ **IMU Sensor** - Fixed Gazebo sensor lookup name (`imu_sensor` not `iris::base_link::imu_sensor`)
- ✅ **Motor Thrust** - Increased to 5N/motor (hover at ~200 throttle = 15.7N > 14.7N weight)
- ✅ **Yaw Torque** - Fixed formula: (CCW motors - CW motors) = (M2+M3) - (M1+M4)
- ✅ **Angular Damping** - Roll/pitch -30.0, Yaw -50.0; Linear drag -10.0; Position spring 10.0 N/m + velocity damping 8.0 N/(m/s)
- ✅ **Ground Effect** - Up to 30% thrust boost below 1m altitude
- ✅ **Bridge UDP** - IMU receiver on port 9002, motor forward on port 9003
- ✅ **Synthetic IMU Fallback** - Auto-switch with warning when Gazebo data unavailable
- ✅ **Altitude Hold** - Accel Z PID (target 1g = 16384 LSB), throttle clamp 150-240
- ✅ **Yaw Angle Control** - Cascade angle→rate PID (kp 15, ki 1, kd 6)
- ✅ **Position Hold** - Spring 5.0 N/m + velocity damping 3.0 N/(m/s) to origin
- ✅ **Motor Clamping** - 0-255 range enforced in write_motors()

### Feedback Controller (programs/feedback_controller.c)
- **Complementary Filter**: 98% gyro + 2% accel for roll/pitch attitude (fixed atan2 approx)
- **Cascade PID**: Angle (outer kp 20) → Rate (inner kp 15) for roll/pitch
- **Altitude PID**: Uses accel Z (target 16384 = 1g hover), throttle clamp 150-240
- **Yaw Angle Hold**: Angle (outer kp 20) → Rate (inner kp 15) cascade
- **Mixer**: X-configuration (M1=RR CW, M2=FR CCW, M3=RL CCW, M4=FL CW) with correct yaw signs
- **Test Sequence**: Step targets at 2000-cycle intervals (roll/pitch ±2°, yaw ±5°)
- **Telemetry**: Every 20 cycles (attitude, rates, AZ, throttle, motor outputs)

## Notes
- Rust edition 2024, requires rustc ≥ 1.85
- Cross-compile test programs with `riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32`
- Gazebo SITL integration: External Python bridges via Unix sockets (core emulator is Gazebo-free)
- Flight controller uses SPI for MPU-6000 IMU + GPIO for motor outputs

## VEGA Pilot Firmware Progress (as of 2026-09-28)

### Milestone 0: Audit and Scaffolding - ✅ COMPLETE
- **AUDIT.md** created at `docs/AUDIT.md` with full emulator analysis
- **firmware/** directory structure created per spec Section 3
- **Makefile** builds both RV32 target (`build/rv32/firmware.elf`) and host test target (`build/host/test_runner`)
- **Headers created**: config.h, types.h, sched.h, state.h, ctrl_stub.h, hal/*.h, hal_vega/regs.h
- **HAL interfaces defined**: imu.h, baro.h, mag.h, gnss.h, adc.h, pwm.h, rc.h, uart.h, gpio.h, flash.h, time.h
- **Emulator HAL implementations**: time.c, uart.c, gpio.c, spi.c (IMU), i2c.c (baro/mag), adc.c, pwm.c, gnss.c, rc.c
- **Core modules**: main.c (scheduler + tasks), sched.c, state/state.c, ctrl_stub/ctrl_stub.c (placeholder)
- **Host unit tests**: 5 tests passing (scheduler, state init, ctrl stub, setpoints, param bounds)
- **Verified**: `make rv32` builds ELF; `make test` runs and passes; emulator runs firmware for 100K cycles

### Firmware Build Configuration
```
# Toolchain
RV_CC := riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32
HOST_CC := gcc -m64

# Memory budget
SRAM_BUDGET_BYTES := 256 KB (real hardware)
Stack reserve := 32 KB

# Scheduler rates (from config.h)
Control loop:      1000 Hz (1000 µs)
State estimate:    1000 Hz
Mode/Failsafe:     50 Hz
MAVLink RX:        100 Hz
MAVLink TX:        10 Hz
Logger:            10 Hz
```

### Project Structure (firmware/)
```
firmware/
├── Makefile
├── link.ld              # 256 KB RAM at 0x8000_0000
├── crt0.s               # Stack at 0x8004_0000
├── include/
│   ├── config.h         # All tunables, limits, defaults
│   ├── types.h          # Common types (status_t, time_us_t, etc.)
│   ├── sched.h          # sched_task_t, sched_run()
│   ├── state.h          # vehicle_state_t, setpoints_t, motor_out_t, ctrl_interface_t
│   ├── ctrl_stub.h      # Placeholder controller interface
│   ├── hal/             # HAL interface headers (no impl)
│   │   ├── imu.h, baro.h, mag.h, gnss.h, adc.h
│   │   ├── pwm.h, rc.h, uart.h, gpio.h, flash.h, time.h
│   ├── hal_vega/regs.h  # Single source of truth for register addresses
│   └── hal/time_host.h  # Host test time mock helpers
├── src/
│   ├── main.c           # Scheduler loop, task registration
│   ├── sched.c          # Cooperative fixed-rate scheduler
│   ├── state/state.c    # Vehicle state + sensor update stubs
│   ├── ctrl_stub/ctrl_stub.c  # Placeholder controller (to be replaced by human)
│   ├── hal_emu/         # Emulator-backed HAL implementations
│   │   ├── time.c       # CLINT mtime + CSR mcycle (hi/lo/hi read)
│   │   ├── uart.c       # 16550-style blocking TX, non-blocking RX
│   │   ├── gpio.c       # GPIO via MMIO
│   │   ├── spi.c        # IMU (MPU-6000/ICM-20602) via SPI
│   │   ├── i2c.c        # MS5611 baro + HMC5883L mag via I2C
│   │   ├── adc.c        # Battery, current, RSSI via ADC
│   │   ├── pwm.c        # ESC outputs via PWM
│   │   ├── gnss.c       # NMEA parser stub
│   │   └── rc.c         # SBUS/CRSF parser stub
│   └── hal_vega/        # Placeholder for real hardware HAL
├── tests/host/test_main.c  # 5 unit tests
└── tools/               # For uart_tcp_bridge.py (M4)
```

### Key Design Decisions
1. **Pure C11, no stdlib** - `-nostdlib -fno-builtin`, links libgcc for soft-float
2. **Portable core** - All logic modules compile on host (x86-64) and target (RV32)
3. **Static allocation only** - No malloc/free, fixed-size buffers in config.h
4. **HAL abstraction** - `hal_emu/` for emulator, `hal_vega/` for real hardware
5. **Register addresses** only in `include/hal_vega/regs.h` (from emulator memory map)
6. **Controller seam** - `ctrl_interface_t` with init/update/reset; stub keeps demos running

### Next: Milestone 1 (Scheduler, Time, Profiling)
- [ ] UART ring buffers (non-blocking TX with backpressure)
- [ ] Cycle profiling via `mcycle` - per-task min/avg/max/overruns
- [ ] Safe 64-bit time reads (hi/lo/hi re-read loop)
- [ ] Microbenchmark table: float + - * /, sqrtf, sinf, cosf, atan2f cycles
- [ ] Demo with dummy tasks at different rates on emulator

## Gazebo Peripheral Exposure Status (as of 2026-09-27)

### Currently Exposed to Gazebo (7/12 peripherals)

| Peripheral | Emulator Socket | Python Bridge | Gazebo Plugin | UDP Port | Status |
|------------|----------------|---------------|---------------|----------|--------|
| **SPI** (IMU) | `/tmp/thejas_spi.sock` | MPU-6000 register emulation | ❌ Direct access | 9002 (IMU data) | ✅ Working |
| **GPIO** (Motors) | `/tmp/thejas_gpio.sock` | 4-byte motor forward | 4 motor forces | 9003 (motor cmds) | ✅ Working |
| **I2C** (Sensors) | `/tmp/thejas_i2c.sock` | Baro/Compass/GPS emulation | Baro/Compass/GPS sensors | 9002 (I2C data) | ✅ Working |
| **UART0** (Telemetry) | `/tmp/thejas_uart0.sock` | GPS NMEA, telemetry | GPS NMEA output | 9002 (UART data) | ✅ Working |
| **UART1** (GNSS) | `/tmp/thejas_uart1.sock` | GPS NMEA | GPS NMEA output | 9002 (UART data) | ✅ Working |
| **UART2** (RC) | `/tmp/thejas_uart2.sock` | RC SBUS/CRSF | N/A | 9002 (UART data) | ✅ Working |
| **ADC** (Analog) | `/tmp/thejas_adc.sock` | Battery/Current/RSSI | Custom sensor emulation | 9002 (ADC data) | ✅ Working |
| **PWM** (Servos) | `/tmp/thejas_pwm.sock` | 8-channel duty/freq/phase | Motor/servo joints | 9002 (PWM data) | ✅ Working |

### Not Yet Exposed (5/12 peripherals)

| Peripheral | Emulator Socket | Plan | Gazebo Sensor | Priority |
|------------|----------------|------|---------------|----------|
| **CAN** | Add `/tmp/thejas_can.sock` | ESC telemetry, GPS | N/A | 1 |
| **Timer** | Local only | Timer capture/compare | N/A | 2 |
| **QSPI** | Local only | Flash XIP | N/A | - |
| **DMA** | Local only | Memory copy | N/A | - |
| **WDT** | Local only | Watchdog | N/A | - |
| **I2S** | Local only | Audio | N/A | - |

### Next Implementation Plan (One by One)

1. ✅ **I2C Bridge** - Barometer (MS5611), Compass (HMC5883L), GPS (I2C) - DONE
2. ✅ **UART Bridge** - GPS NMEA, Telemetry, Serial console - DONE
3. ✅ **ADC Bridge** - Battery voltage, Current sensor, RSSI - DONE
4. ✅ **PWM Bridge** - 8+ servo/ESC outputs - DONE
5. **CAN Bridge** - ESC telemetry, UAVCAN

---

## M0 Addendum (Before Starting M1)

### 1. Finish Scaffolding Verification
- [x] Verify Makefile builds RV32 ELF + host test target with libgcc
- [x] Linker: RAM LENGTH=256K, derive SP from `__stack_top` symbol
- [x] Build fails if static RAM (.bss + .data) + 32 KB stack > SRAM_BUDGET_BYTES (256 KB)
- [x] config.h, types.h, all HAL headers complete

### 2. Sensor-Coverage Matrix for AUDIT.md
Add table: what Gazebo/bridge actually simulates for each sensor:
| Sensor | Emulator/IPC | Gazebo Plugin | Python Bridge | Notes |
|--------|--------------|---------------|---------------|-------|
| IMU (MPU-6000/ICM-20602) | SPI socket + virtual regs | IMU sensor (7×double UDP 9002) | sitl_bridge.py spi_imu_server | Synthetic fallback |
| Baro (MS5611) | I2C socket + virtual EEPROM | Baro sensor | sitl_bridge.py i2c_sensor_server | Pressure/temp via I2C |
| Mag (HMC5883L) | I2C socket + virtual EEPROM | Compass sensor | sitl_bridge.py i2c_sensor_server | 3-axis via I2C |
| GNSS (NMEA) | UART socket | GPS NMEA output | sitl_bridge.py uart_server | NMEA sentences |
| RC Input | UART socket | N/A | N/A | SBUS/CRSF/PPM parser in firmware |
| Battery/ADC | ADC socket | Custom analog sensor | sitl_bridge.py | Voltage, current, RSSI |

### 3. Proposed Emulator Changes — **DONE (Patches 1-4)**
- [x] **3+ UART instances**, each with own Unix socket (`/tmp/thejas_uart0.sock`, `uart1.sock`, `uart2.sock`)
- [x] **`--strict-mem` flag**: access fault on unmapped loads/stores; treat PC=0 as fault (not clean exit)
- [x] **I2C IPC improvements**: command-only writes; multi-byte reads (MS5611 PROM, HMC5883L data registers)
- [x] **Clamp synthetic ADC** to 4095 (12-bit); fix `mcycleh` at 0xB80 (keep 0xC80 as user shadow)
- [ ] **Configurable latency** for mul/div/load in cycle counter (optional)

### 4. Time Base & Profiler Updates — **DONE**
- [x] `hal_time_us()` from CLINT `mtime` with `MTIME_HZ` in config.h (emulator = 100 MHz nominal)
- [x] Use `mtimecmp` for scheduler tick (not Timer peripheral)
- [x] Profiler output labeled **"instructions retired"** (not "cycles" - emulator ≈ 1:1, real HW has pipeline)

### 5. Open Item: Real Boot Flow
- [ ] Document: XIP (execute from QSPI at 0x4000_0000) vs copy-to-RAM
- [ ] Report memory budget under both assumptions:
  - XIP: .text in flash, only .data/.bss in RAM
  - Copy-to-RAM: entire image in SRAM

### 6. Emulator Version
- [x] Replace "emulator version" in AUDIT.md with real git commit hash from Thejas32rs repo (`2702f25`)