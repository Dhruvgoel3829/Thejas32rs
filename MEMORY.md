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

## Current State (as of commit 3b2e1d4)

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
- `flight_controller.c` - SPI IMU + PID control loop
- `dma_test.c`, `i2c_test.c`, `wdt_test.c`, `pmp_test.c`, `mixer_test.c`, `imu_test.c`, `comprehensive_test.c`, `xip_test.c`, `peripherals_test.c`, `echo.c`, `shell.c`

### Build & Run
```bash
cargo build --release
./target/release/Thejas32rs <elf> [--trace] [--dump-regs] [--max-cycles N] [--gdb-port N]
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

## Known Gaps / Unimplemented

### CPU Instructions (src/cpu/mod.rs)
1. **LOAD funct3=3,6,7** - FLW, reserved (line 495)
2. **STORE funct3=3-7** - FSW, reserved (line 518)
3. **R/M type** - Some MUL/DIV/REM combos (line 441)
4. **AMO funct5** - Some variants (line 627)
5. **Unknown opcodes** - Catch-all (line 635)

### Recently Fixed (2026-09-25)
- ✅ **System funct12=0x121** - SFENCE.VMA (implemented as no-op, no virtual memory)
- ✅ **CSR funct3=0x4** - Reserved SYSTEM funct3 now traps with illegal instruction (cause 2)
- ✅ **ELF Loading Bug** - Fixed PT_LOAD segment handling for segments overlapping RAM (including BSS-only segments with filesz=0). All test programs now load correctly.

### Missing Architecture Features
- RV32F/D (FPU) - not in scope (RV32IM only)
- Supervisor/User mode - Machine mode only
- Vector/Hypervisor extensions - not implemented

### Dead/Unused Code
- `src/ipc.rs` - SocketBackend, Gazebo IMU backend (unused - Unix socket IPC for Python bridges)
- `RawModeGuard` in main.rs - Never constructed
- Unused constants: `*_SIZE` in all peripheral modules, `CSR_PMPCFG1/2/3` in csr.rs

### Testing
- **Zero unit tests** (cargo test runs 0 tests)
- Integration testing only via manual ELF execution

## Project Structure
```
Thejas32rs/
├── Cargo.toml              # edition 2024, deps: goblin, crossterm, clap
├── .gdbinit                # GDB config
├── .gitignore              # Cargo artifacts
├── src/
│   ├── main.rs             # CLI, execution loop, GDB integration
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
│   ├── uart.rs             # UART + crossterm polling thread
│   ├── gpio.rs             # GPIO
│   ├── timer.rs            # Programmable timer
│   ├── spi.rs              # SPI controller
│   ├── i2s.rs              # I²S audio
│   ├── i2c.rs              # I²C + virtual EEPROM
│   ├── dma.rs              # DMA engine
│   ├── wdt.rs              # Watchdog timer
│   ├── qspi.rs             # QSPI flash window
│   ├── gdb.rs              # GDB stub protocol
│   ├── ipc.rs              # Socket IPC (UNUSED)
│   └── gazebo_backend.rs   # Gazebo UDP (UNUSED)
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
└── tools/                  # Python SITL bridges
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
1. Remove dead code (ipc.rs, unused constants)
2. Add unit tests for CPU instructions and peripherals
3. Complete remaining unimplemented instructions (FPU loads/stores, some MUL/DIV/REM, AMO variants)
4. Consider FPU if RV32IMF desired
5. Build and test Gazebo plugin (`gazebo/build/`) with `gzserver`

## Gazebo SITL Integration (AI-in-the-Loop Drone Firmware Development)

### Architecture
```
┌─────────────────┐     UDP 9002 (IMU)     ┌──────────────────┐
│  Thejas32rs     │ ◄────────────────────── │  Gazebo Plugin   │
│  (Rust Emulator)│                         │  (C++ ModelPlugin)│
│                 │     UDP 9003 (Motors)   │                  │
│  SPI ←→ IMU     │ ──────────────────────► │  4 Rotor Joints  │
│  GPIO ←→ Motors │                         │  IMU Sensor      │
└─────────────────┘                         └──────────────────┘
```

### Components
| Component | File | Status |
|-----------|------|--------|
| Gazebo C++ Plugin | `gazebo/gazebo_thejas_sitl.cpp` | ✅ Built as `libthejas_sitl.so` |
| Gazebo World | `gazebo/worlds/iris.world` | ✅ Quadcopter with IMU + 4 rotors |
| Rust IMU Backend | `src/gazebo_backend.rs` | ✅ UDP port 9002 (fixed bind) |
| Rust Motor Backend | `src/gazebo_backend.rs` | ✅ UDP port 9003 |
| SPI Peripheral | `src/spi.rs` | ✅ Conditional Gazebo backend |
| GPIO Peripheral | `src/gpio.rs` | ✅ Conditional Gazebo backend |
| CLI Flag | `src/main.rs` | ✅ `--gazebo` flag |

### UDP Protocol
| Port | Direction | Payload | Description |
|------|-----------|---------|-------------|
| 9002 | Gazebo → Rust | 7×double (56B) | IMU: timestamp, ax, ay, az, gx, gy, gz |
| 9003 | Rust → Gazebo | 4×float (16B) | Motor commands: 0.0-1.0 normalized |
| 9004 | Rust → Gazebo | 1×byte | Telemetry (unused in plugin) |

### Usage
```bash
# Terminal 1: Start Gazebo with Iris world
cd gazebo/build && gzserver ../worlds/iris.world

# Terminal 2: Run emulator with flight controller in Gazebo mode
./target/release/Thejas32rs programs/flight_controller.elf --gazebo
```

### Python Bridges (Alternative Unix Socket Architecture)
- `tools/sitl_bridge.py` - Full bridge: Gazebo CLI ↔ Unix sockets ↔ Emulator
- `tools/motor_bridge.py` - Motor monitor via `/tmp/thejas_gpio.sock`
- `tools/external_imu_proxy.py` - Synthetic IMU via `/tmp/thejas_spi.sock`

### Fixed Issues (2026-09-25)
- ✅ **GazeboImuBackend bind bug** - Now binds to port 9002 (was ephemeral port 0)
- ✅ **Conditional Gazebo backend** - `--gazebo` flag enables UDP backends
- ✅ **Graceful fallback** - Standard mode works without Gazebo running

## Notes
- Rust edition 2024, requires rustc ≥ 1.85
- Cross-compile test programs with `riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32`
- Gazebo SITL integration: UDP-based (C++ plugin) + optional Python Unix socket bridges
- Flight controller uses SPI for MPU-6000 IMU + GPIO for motor outputs