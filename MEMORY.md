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
- `plic_test.c` - PLIC interrupt test
- `scheduler_test.c` + `scheduler_asm.s` - Preemptive scheduler with trap entry
- `flight_controller.c` - SPI IMU + PID control loop (simple rate PID)
- `feedback_controller.c` - Full cascade PID (angle→rate) + complementary filter
- `drone_test.c` - System test: IMU, motor sequence, hover ramp
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
- **115 unit tests** passing (cargo test)
  - CPU decode: 22 tests (RV32I, M, A, C, System mnemonics)
  - CSR: 10 tests (read/write, MISA, cycle, PMP config/addr)
  - PMP: 18 tests (config/addr R/W, TOR/NA4/NAPOT, access checks)
  - Peripherals: 65 tests (CLINT, PLIC, Timer, GPIO, SPI, I2S, I2C, DMA, WDT)
- Integration testing via manual ELF execution (hello, compressed, atomic, timer, scheduler, flight controller, etc.)

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
✅ 115 unit tests
✅ Illegal instruction traps (cause 2)

No remaining tasks within THEJAS32 scope. Project is feature-complete for its defined ISA.

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

## Gazebo Peripheral Exposure Status (as of 2026-09-27)

### Currently Exposed to Gazebo (6/12 peripherals)

| Peripheral | Emulator Socket | Python Bridge | Gazebo Plugin | UDP Port | Status |
|------------|----------------|---------------|---------------|----------|--------|
| **SPI** (IMU) | `/tmp/thejas_spi.sock` | MPU-6000 register emulation | ❌ Direct access | 9002 (IMU data) | ✅ Working |
| **GPIO** (Motors) | `/tmp/thejas_gpio.sock` | 4-byte motor forward | 4 motor forces | 9003 (motor cmds) | ✅ Working |
| **I2C** (Sensors) | `/tmp/thejas_i2c.sock` | Baro/Compass/GPS emulation | Baro/Compass/GPS sensors | 9002 (I2C data) | ✅ Working |
| **UART** (Telemetry) | `/tmp/thejas_uart.sock` | GPS NMEA, telemetry | GPS NMEA output | 9002 (UART data) | ✅ Working |
| **ADC** (Analog) | `/tmp/thejas_adc.sock` | Battery/Current/RSSI | Custom sensor emulation | 9002 (ADC data) | ✅ Working |
| **PWM** (Servos) | `/tmp/thejas_pwm.sock` | 8-channel duty/freq/phase | Motor/servo joints | 9002 (PWM data) | ✅ Working |

### Not Yet Exposed (6/12 peripherals)

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
4. **PWM Bridge** - 8+ servo/ESC outputs
5. **CAN Bridge** - ESC telemetry, UAVCAN