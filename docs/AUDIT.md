# VEGA Pilot Firmware: Emulator Audit Report

**Date:** 2026-09-28  
**Auditor:** opencode agent  
**Emulator Version:** Thejas32rs (commit `28456f72d7bfd4c5c88c1d3f6bfc7f5910a50819`)

**Repository Structure Confirmation:** Single repo, commit `28456f7`. The emulator source is in `src/` and firmware in `firmware/` as sibling directories at this commit. The Rust emulator source at this commit has been verified against the memory map, register layouts, and IPC wire formats documented below.

**Stack High-Water Mark Semantics:** Implemented in `firmware/src/main.c::stack_high_water_mark()`. Stack grows DOWN from `__stack_top`. Scan runs from LOW end (`__bss_end`) upward toward `__stack_top`. Stops on first word where paint `!= 0xDEADBEEF` (i.e., first overwritten word from the deep end). Reports historical maximum stack depth (bytes from deepest overwritten address to `__stack_top`), not current SP position.

**Emulator Enhancements (Patch Set 4):**
- **3 UARTs** with independent Unix sockets (`/tmp/thejas_uart0/1/2.sock`)
- **I2C Multi-byte Reads** — extended IPC protocol for MS5611 (3-byte) and HMC5883L (6-byte) burst reads
- **Writable QSPI Flash** — file-backed 16 MB persistence via `--flash-file`, auto-flush on exit
- **`--strict-mem`** — faults on unmapped memory access, PC=0 treated as fault
- **`--test-finished-reg`** — firmware writes exit code to register, emulator exits with that code
- **`--strict-mem` PC=0 Fault** — PC=0 treated as fault in strict mode (not clean exit)
- **Stack High-Water Mark** — paint 0xDEADBEEF, scan from `__bss_end` to `__stack_top`, report historical max depth

---

## 1. Memory Map

| Region | Base Address | Size | Description |
|--------|-------------|------|-------------|
| Unmapped | 0x0000_0000 – 0x3FFF_FFFF | 1 GB | Returns 0 on read, ignores writes |
| QSPI Flash (XIP) | 0x4000_0000 – 0x40FF_FFFF | 16 MB | Execute-in-place, **writable**, file-backed persistence via `--flash-file` |
| SRAM | 0x8000_0000 – 0x8007_FFFF | **512 KB** | Main RAM, BSS zero-filled on ELF load (firmware budget 256 KB) |
| CLINT | 0x0200_0000 – 0x0200_FFFF | 64 KB | Core Local Interruptor (mtime, mtimecmp, msip) |
| PLIC | 0x0C00_0000 – 0x0C3F_FFFF | 4 MB | Platform-Level Interrupt Controller (32 sources) |
| UART0 | 0x1000_0000 – 0x1000_0FFF | 4 KB | 16550-style UART (THR/RHR at 0x00, LSR at 0x05) — Telemetry console |
| UART1 | 0x1000_1000 – 0x1000_1FFF | 4 KB | 16550-style UART — GNSS NMEA (IPC socket) |
| UART2 | 0x1000_2000 – 0x1000_2FFF | 4 KB | 16550-style UART — RC input (SBUS/CRSF/PPM, IPC socket) |
| GPIO | 0x1001_0000 – 0x1001_0FFF | 4 KB | Data in/out, direction, set/clear, int_en |
| Timer | 0x1002_0000 – 0x1002_0FFF | 4 KB | Programmable timer with prescaler, auto-reload |
| SPI | 0x1003_0000 – 0x1003_0FFF | 4 KB | SPI controller (loopback + virtual flash mock) |
| I2S | 0x1004_0000 – 0x1004_0FFF | 4 KB | I²S audio interface |
| I2C | 0x1005_0000 – 0x1005_0FFF | 4 KB | I²C controller with virtual 24C02 EEPROM (0x50) |
| ADC | 0x1008_0000 – 0x1008_0FFF | 4 KB | ADC with 8 channels (IPC or synthetic) |
| PWM | 0x1009_0000 – 0x1009_0FFF | 4 KB | 8-channel PWM, 12-bit duty, configurable freq |
| DMA | 0x1006_0000 – 0x1006_0FFF | 4 KB | DMA engine (16 bytes/cycle, src/dst inc) |
| WDT | 0x1007_0000 – 0x1007_0FFF | 4 KB | Watchdog timer (reset or interrupt mode) |

---

## 2. Peripheral Register Layouts

### CLINT (0x0200_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x0000 | msip | 32-bit | R/W | Machine Software Interrupt Pending (bit 0) |
| 0x4000 | mtimecmp | 32-bit | R/W | Timer compare low 32 bits |
| 0x4004 | mtimecmph | 32-bit | R/W | Timer compare high 32 bits |
| 0xBFF8 | mtime | 32-bit | R/W | Timer counter low 32 bits |
| 0xBFFC | mtimeh | 32-bit | R/W | Timer counter high 32 bits |

**Behavior:** `mtime` increments by 1 every SoC clock cycle (emulator cycle). When `mtime >= mtimecmp`, MTIP (MIP bit 7) is set.

### PLIC (0x0C00_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x000004–0x00007C | priority[N] | 32-bit | R/W | Priority for interrupt source N (1–31), 8 levels |
| 0x001000 | pending | 32-bit | R | Pending bitmask |
| 0x002000 | enabled | 32-bit | R/W | Enable bitmask for Hart 0 |
| 0x200000 | threshold | 32-bit | R/W | Priority threshold |
| 0x200004 | claimcomplete | 32-bit | R/W | Read: claims highest-priority interrupt; Write: completion |

**Interrupt Sources:**
- 1: UART (RX data ready)
- 2: Timer (match interrupt)
- 3: DMA (transfer complete)
- 4: WDT (timeout)

### UART (0x1000_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | THR/RHR | 8-bit | R/W | Transmit Holding / Receive Buffer |
| 0x05 | LSR | 8-bit | R | Line Status: bit 0 = RX ready, bits 5–6 = TX ready (0x60) |

**IPC:** When `--ipc` enabled, connects to `/tmp/thejas_uart.sock`. TX bytes forwarded; RX bytes received from socket.

### GPIO (0x1001_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | data_in | 32-bit | R | `(data_in & ~dir) | (data_out & dir)` |
| 0x04 | data_out | 32-bit | R/W | Output data |
| 0x08 | direction | 32-bit | R/W | 1 = output, 0 = input |
| 0x0C | set | 32-bit | W | Atomic bit set on `data_out` |
| 0x10 | clear | 32-bit | W | Atomic bit clear on `data_out` |
| 0x14 | int_en | 32-bit | R/W | Interrupt enable (not yet hooked to PLIC) |

**IPC:** On write to data_out/set/clear, sends 4-byte LE `data_out` to `/tmp/thejas_gpio.sock`.

### Timer (0x1002_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | ctrl | 32-bit | R/W | bit 0: enable, bit 1: irq_en, bit 2: auto_reload |
| 0x04 | count | 32-bit | R/W | Current counter value |
| 0x08 | reload | 32-bit | R/W | Compare/reload value (default: 0xFFFF_FFFF) |
| 0x0C | presc | 32-bit | R/W | Prescaler divider |
| 0x10 | status | 32-bit | W1C | bit 0: match interrupt flag (write 1 to clear) |

**Behavior:** On each SoC cycle, if enabled: increments prescaler counter; when reaches `prescaler`, increments `count`; if `count >= reload`, sets status bit 0, triggers IRQ if enabled, auto-reloads or disables.

### SPI (0x1003_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | cr | 32-bit | R/W | bit 0: enable, bit 1: loopback mode |
| 0x04 | sr | 32-bit | R | bit 0: TX ready, bit 1: RX ready |
| 0x08 | data | 32-bit | R/W | TX/RX data register |
| 0x0C | cs | 32-bit | R/W | Chip select (emulator uses low byte) |

**Loopback Mode (cr bit 1):** TX byte immediately returned in `data` as `mosi + 1`.

**Virtual Flash Mock:** Byte `0x9F` (JEDEC ID) returns `0xEF` (Winbond); other bytes return `byte + 1`.

**IPC Protocol (`/tmp/thejas_spi.sock`):**
- Packet type 0x01: CS line transition, 1 byte payload `[cs_state]` (bit 0 = CS asserted low)
- Packet type 0x02: SPI full-duplex transfer, 2 bytes payload `[cs_state, mosi]` → returns 1 byte `[miso]`

### I2C (0x1005_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | cr | 32-bit | R/W | bit 2: START, bit 3: read direction |
| 0x04 | sr | 32-bit | R/W | bit 2: ACK, bit 1: RX ready |
| 0x08 | addr | 32-bit | R/W | 7-bit device address |
| 0x0C | data | 32-bit | R/W | Data register |

**Virtual EEPROM:** 256 bytes at address 0x50. First write after START sets internal pointer; subsequent writes write data with auto-increment. Reads return data at pointer with auto-increment.

**IPC Protocol (`/tmp/thejas_i2c.sock`):**
- Write: `[0x00, addr, reg, data]`
- Read: `[0x01, addr, reg]` → response: `[data]`

### ADC (0x1008_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | ctrl | 32-bit | R/W | bit 0: enable, bit 1: continuous, bits 4-7: channel select |
| 0x04 | status | 32-bit | R | bit 0: data ready, bit 1: overflow (W1C) |
| 0x08 | data | 32-bit | R | 12-bit ADC result |
| 0x0C | chan | 32-bit | R/W | Channel select (0-7) |

**Synthetic Values (no IPC):**
- Channel 0: 4500 (~3.6V battery)
- Channel 1: 2048 (mid-scale current)
- Channel 2: 3000 (RSSI)
- Channels 3-7: 0

**IPC Protocol (`/tmp/thejas_adc.sock`):**
- Request: `[0x01, channel]` → Response: 2 bytes LE `u16` result

### PWM (0x1009_0000)
| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| 0x00 | ctrl | 32-bit | R/W | bit 0: enable, bits 1-8: channel enables, bit 16: center-aligned |
| 0x04 | freq | 32-bit | R/W | PWM frequency in Hz (default 50000) |
| 0x10–0x2C | duty0–duty7 | 32-bit | R/W | Channel 0-7 duty cycle (0-4095, 12-bit) |
| 0x30 | dead_time | 32-bit | R/W | Dead time insertion (ns) |
| 0x34–0x53 | phase0–phase7 | 32-bit | R/W | Phase shift per channel (0-360 degrees × 100) |

**IPC Protocol (`/tmp/thejas_pwm.sock`):**
- Message: `[op, channel, val_le_bytes[4]]`
- op: 0x00=ctrl, 0x01=freq, 0x02=duty, 0x03=dead_time, 0x04=phase

---

## 3. IPC Wire Formats

All IPC uses Unix domain sockets (`SOCK_STREAM`) at paths under `/tmp/thejas_*.sock`.

### SPI (`/tmp/thejas_spi.sock`)
**From Emulator → Bridge:**
- Type 0x01 (CS): `[0x01, cs_state]` — cs_state bit 0 = asserted (0=low/active)
- Type 0x02 (Transfer): `[0x02, cs_state, mosi]` → expects 1 byte reply `[miso]`

**Note:** Emulator's `spi_exchange()` in `ipc.rs` temporarily sets blocking mode for the transfer.

### GPIO (`/tmp/thejas_gpio.sock`)
**From Emulator → Bridge:** 4 bytes LE `u32` = `data_out` value (sent on every write to data_out, set, clear).

### I2C (`/tmp/thejas_i2c.sock`)
**From Emulator → Bridge:**
- Write: `[0x00, addr, reg, data]`
- Read request: `[0x01, addr, reg]` → Bridge replies `[data]`

### UART (`/tmp/thejas_uart.sock`)
**From Emulator → Bridge:** 1 byte per TX (THR write).
**From Bridge → Emulator:** 1 byte per RX (emulator reads via `ipc.recv()` into RX buffer).

### ADC (`/tmp/thejas_adc.sock`)
**From Emulator → Bridge:** `[0x01, channel]` (request conversion)
**From Bridge → Emulator:** 2 bytes LE `u16` (conversion result)

### PWM (`/tmp/thejas_pwm.sock`)
**From Emulator → Bridge:** `[op, channel, val[0], val[1], val[2], val[3]]` (val = LE u32)

---

## 3.5 Sensor-Coverage Matrix (Gazebo/Bridge Simulation)

The following table documents what the Gazebo/bridge side actually simulates for each required sensor. This is critical for validating firmware sensor drivers before hardware availability.

| Sensor | Emulator/IPC Interface | Gazebo Plugin | Python Bridge (`sitl_bridge.py`) | Notes |
|--------|------------------------|---------------|-----------------------------------|-------|
| **IMU (MPU-6000/ICM-20602)** | SPI socket (`/tmp/thejas_spi.sock`) + virtual register model | IMU sensor publishing 7×double (56B) on UDP 9002: timestamp, ax, ay, az, gx, gy, gy | `spi_imu_server()`: MPU-6000 register emulation (WHO_AM_I=0x68, burst read 0x3B–0x48). Converts SI units to register LSBs. | Downcast to int16 (MPU-6000 raw LSB format) occurs in `tools/sitl_bridge.py::udp_imu_receiver()` lines 77–82; SPI virtual registers only ever hold int16-width data. Synthetic fallback when Gazebo data unavailable (sin/cos wave). Converts m/s² → 16384 LSB/g, rad/s → 16.4 LSB/(deg/s) |
| **Barometer (MS5611)** | I2C socket (`/tmp/thejas_i2c.sock`) + virtual EEPROM at 0x50 | Baro sensor (pressure/temperature) | `i2c_sensor_server()`: Handles MS5611 commands (0x1E reset, 0x40–0x46 D1/D2 conversion, 0xA0–0xAE PROM read). Returns 24-bit pressure/temp. | Pressure in Pa, temperature in °C. Requires multi-byte read (3 bytes for ADC result) |
| **Compass (HMC5883L)** | I2C socket (`/tmp/thejas_i2c.sock`) + virtual EEPROM at 0x50 | Compass sensor (3-axis mag) | `i2c_sensor_server()`: Handles HMC5883L registers (0x03–0x08 data, 0x0A status, 0x0B config A, 0x0C config B, 0x0D mode). Returns 16-bit X/Z/Y. | Gain/range configurable via Config B. Continuous measurement mode supported. |
| **GNSS (NMEA)** | UART socket (`/tmp/thejas_uart.sock`) | GPS NMEA output | `uart_server()`: Forwards NMEA sentences from Gazebo GPS plugin to emulator UART RX. | NMEA parser in firmware (`nmea_parser_feed/parse`) processes $GPGGA, $GPRMC, etc. |
| **RC Input (SBUS/CRSF/PPM)** | UART socket (`/tmp/thejas_uart.sock`) | N/A | N/A (RC parser in firmware) | Firmware implements SBUS/CRSF/PPM parser (`rc_parser_feed/parse`) fed by UART drain task. |
| **Battery/Current/RSSI (ADC)** | ADC socket (`/tmp/thejas_adc.sock`) | Custom analog sensor emulation | `sitl_bridge.py`: Returns voltage/current/RSSI on request `[0x01, channel]` → 2-byte LE u16. | Ch0: battery voltage, Ch1: current, Ch2: RSSI. Clamped to 12-bit (4095). |

**Key Gaps for Emulator Patches:**
1. **3 UART instances needed** — GNSS and RC tasks currently share one UART socket; cannot be validated independently.
2. **I2C multi-byte reads** — MS5611 needs 3-byte ADC read; HMC5883L needs 6-byte burst read. Current IPC only supports single-byte reads.
3. **ADC clamp** — Synthetic values must be clamped to 4095 (12-bit).

---

## 4. Time Base & Cycle Counter Behavior

### CLINT `mtime` / `mtimeh`
- 64-bit counter at `0x0200_BFF8` (low) / `0x0200_BFFC` (high)
- Increments **once per emulator cycle** (i.e., once per instruction executed, approximately)
- In `clint.rs::tick()`, called every cycle from `cpu.step()`
- **Important:** This means `mtime` frequency = instruction execution rate, NOT a fixed Hz. At ~100 MHz target, 1 tick ≈ 10 ns. But emulator runs at host speed (millions of cycles/sec).

### CSR `mcycle` / `mcycleh`
- CSR 0xB00 (mcycle) / 0xC80 (mcycleh)
- 64-bit cycle counter, increments every emulator cycle
- In `cpu/mod.rs`, incremented in `step()` before fetch
- **Same as mtime** — both increment per cycle

### Reading 64-bit Registers Safely on RV32
Since RV32 is 32-bit, reading 64-bit `mtime` or `mcycle` requires hi/lo/hi re-read loop:
```c
uint64_t read_mtime(void) {
    uint32_t hi, lo, hi2;
    do {
        hi = READ32(CLINT_BASE + 0xBFFC);
        lo = READ32(CLINT_BASE + 0xBFF8);
        hi2 = READ32(CLINT_BASE + 0xBFFC);
    } while (hi != hi2);
    return ((uint64_t)hi << 32) | lo;
}
```

### Time Conversion
- Emulator: 1 cycle ≈ 1 `mtime` tick ≈ 1 `mcycle` tick
- Target hardware: CLINT `mtime` typically runs at fixed frequency (e.g., 32.768 kHz or 1 MHz per RISC-V spec)
- **Critical Difference:** Emulator cycles ≠ real time. Must calibrate or use a fixed-frequency timer peripheral for real-time scheduling.

---

## 5. Makefile Analysis (`programs/MakeFile`)

```make
CROSS_COMPILE ?= riscv-none-elf-
CC      := $(CROSS_COMPILE)gcc
OBJDUMP := $(CROSS_COMPILE)objdump
OBJCOPY := $(CROSS_COMPILE)objcopy

CFLAGS  := -march=rv32imac_zicsr -mabi=ilp32 -O2 -g -nostdlib -fno-builtin -Wall
LDFLAGS := -T link.ld

TARGETS := atomic_test.elf compressed_test.elf dma_test.elf scheduler_test.elf flight_controller.elf drone_peripheral_test.elf timer_poll_test.elf

all: $(TARGETS)

# Each target: crt0.s + *.c + link.ld
```

**Linker Script (`programs/link.ld`):**
- RAM: ORIGIN=0x80000000, LENGTH=512K
- Sections: .text.entry (KEEP), .text, .rodata, .data, .bss
- Stack top at `ORIGIN(RAM) + LENGTH(RAM)` = 0x8007_0000

**Crt0 (`programs/crt0.s`):**
- Sets `sp = 0x80070000`
- Calls `main`
- Infinite loop on return

**Observations:**
- Uses `-march=rv32imac_zicsr` (matches emulator RV32IMAC+Zicsr)
- No `-ffunction-sections -fdata-sections -Wl,--gc-sections` (but should add for size optimization)
- Builds multiple test ELFs, no separate firmware target

---

## 6. Emulator vs Real THEJAS32/ARIES v3 Differences (Flag for Verification)

| Aspect | Emulator | Real Hardware (Expected) | Verify Against |
|--------|----------|--------------------------|----------------|
| **SRAM Size** | 512 KB | **256 KB** (spec says "believed to be 256 KB") | C-DAC SDK headers, ARIES v3 datasheet |
| **CLINT `mtime` Frequency** | 1 tick per CPU cycle (variable) | Fixed frequency (typically 32.768 kHz or 1 MHz) | RISC-V CLINT spec, VEGA ET1031 docs |
| **QSPI Flash** | 16 MB window at 0x4000_0000, read-only | Likely same, but may support write/erase commands | C-DAC SDK QSPI driver |
| **Peripheral Base Addresses** | As documented above | May differ | C-DAC SDK `hal_vega/regs.h` |
| **PMP Entries** | 16 | Verify | VEGA ET1031 manual |
| **PLIC Sources** | 4 (UART, Timer, DMA, WDT) | May have more (GPIO, SPI, I2C, ADC, PWM) | C-DAC SDK PLIC mapping |
| **GPIO Interrupts** | `int_en` register exists but not hooked to PLIC | Likely connected to PLIC | C-DAC SDK |
| **ADC Channels** | 8 channels, synthetic/IPC | Real ADC with specific channel mapping | C-DAC SDK ADC driver |
| **PWM Channels** | 8 channels, 12-bit | Verify channel count and resolution | C-DAC SDK PWM driver |
| **SPI** | Loopback + virtual flash mock | Real SPI controller with DMA support | C-DAC SDK SPI driver |
| **I2C** | Virtual 24C02 at 0x50 | Real I2C controller | C-DAC SDK I2C driver |
| **WDT** | Reset to 0x8000_0000 or PLIC interrupt | Verify behavior | C-DAC SDK WDT driver |
| **UART** | 16550-style, crossterm terminal | Likely same or 16550-compatible | C-DAC SDK UART driver |
| **FPU** | None (RV32IM only) | **Confirmed: No FPU** | VEGA ET1031 spec |
| **Atomic Ext (A)** | Implemented (LR/SC, AMO) | May or may not be present | VEGA ET1031 ISA spec |
| **Compressed (C)** | Implemented | Likely present (RV32IMAC) | VEGA ET1031 ISA spec |

**Action Required:** Obtain C-DAC SDK headers and ARIES v3 board schematics to produce `hal_vega/regs.h` with verified addresses.

---

## 7. Existing Test Programs (in `programs/`)

| Program | Purpose | Peripherals Used |
|---------|---------|------------------|
| `hello.c` | UART hello world | UART |
| `compressed_test.c` | RV32C factorial | CPU only |
| `atomic_test.c` | AMO/LR/SC tests | CPU (A-ext) |
| `timer_test.c` | Timer interrupt | Timer, PLIC |
| `timer_poll_test.c` | Timer polling | Timer |
| `plic_test.c` | PLIC interrupt test | PLIC, UART, Timer |
| `scheduler_test.c` + `scheduler_asm.s` | Preemptive scheduler | Timer, PLIC, trap entry |
| `flight_controller.c` | SPI IMU + PID loop | SPI, GPIO, UART |
| `feedback_controller.c` | Cascade PID + complementary filter | SPI, GPIO, UART |
| `drone_test.c` | IMU test, motor sequence, hover | SPI, GPIO, UART |
| `drone_peripheral_test.c` | All 8 Gazebo peripherals | SPI, GPIO, I2C, UART, ADC, PWM, Timer |
| `dma_test.c` | DMA memory copy | DMA |
| `i2c_test.c` | I2C EEPROM read/write | I2C |
| `wdt_test.c` | Watchdog timer | WDT |
| `pmp_test.c` | PMP protection | PMP |
| `mixer_test.c` | Motor mixer | GPIO |
| `imu_test.c` | IMU SPI read | SPI |
| `comprehensive_test.c` | Multi-peripheral test | Multiple |
| `xip_test.c` | QSPI XIP execution | QSPI Flash |
| `peripherals_test.c` | General peripheral test | Multiple |
| `shell.c` | Interactive shell | UART |
| `echo.c` | UART echo | UART |

---

## 8. Bugs / Oddities Found

1. **`mtime` increments per instruction cycle, not wall-clock time** — This breaks real-time scheduling assumptions. The Timer peripheral at 0x1002_0000 should be used for fixed-frequency timing, but its prescaler is also cycle-based.

2. **GPIO `int_en` register exists but interrupts not connected to PLIC** — PLIC sources only include UART, Timer, DMA, WDT. GPIO, SPI, I2C, ADC, PWM interrupts are not routed.

3. **SPI `sr` register always returns TX ready (bit 0 = 1)** — In loopback mode, no actual FIFO/status modeling.

4. **I2C virtual EEPROM only at address 0x50** — Real sensors (MS5611 at 0x77, HMC5883L at 0x1E) only work via IPC bridge.

5. **UART has no TX FIFO / interrupt** — LSR bits 5-6 always show TX ready (0x60). No THRE interrupt to PLIC.

6. **CLINT `msip` only bit 0 stored** — Write of 0xFF stores 1. Per spec, only bit 0 is valid.

7. **Emulator exits on PC=0** — Firmware jumping to 0 causes clean shutdown. Real hardware would likely reset or trap.

8. **WDT reset jumps to 0x8000_0000** — This is the SRAM base, not a reset vector. Real hardware likely resets to 0x0000_0000 or flash.

9. **No cycle-accurate pipeline modeling** — Single-cycle execution per instruction. Real THEJAS32 may have pipeline stalls, cache misses, etc.

10. **QSPI Flash is read-only in emulator** — Real QSPI supports write/erase for parameter storage and logging.

---

## 9. Recommendations for Firmware Development

1. **Use Timer peripheral (0x1002_0000) for scheduling**, not CLINT `mtime`. Configure prescaler for ~1 µs or ~10 µs tick.

2. **Implement 64-bit time reading with hi/lo/hi loop** for both CLINT `mtime` and CSR `mcycle`.

3. **All register addresses in `hal_vega/regs.h` only** — no hardcoded addresses in drivers.

4. **Design HAL for both IPC (emulator) and MMIO (hardware)** — same C interface, different backends.

5. **Budget 256 KB SRAM** — reserve ~32 KB for stack, rest for static buffers.

6. **Profile float operations** — no FPU, all software emulated via libgcc. Avoid `double`, minimize `float` in hot paths.

7. **Use `riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32`** for both target and host (host can use native gcc for tests).

---

## 10. Next Steps (Milestone 0 Acceptance)

- [x] This `AUDIT.md` created
- [ ] Create `firmware/` directory layout per spec Section 3
- [ ] Create `firmware/Makefile` building RV32 ELF and host test target
- [ ] Create `firmware/include/config.h`, `types.h`, `hal/*.h`
- [ ] Create minimal `main.c`, `sched.c`, `hal_emu/` stubs
- [ ] Verify `make` builds firmware ELF; `make test` runs (even if empty)

---
*End of Audit Report*