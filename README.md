# Thejas32rs

[![Rust](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/rustc-edition2024-blue)](https://doc.rust-lang.org/edition2024/edition2024-rustdoc/)

A cycle-accurate **RV32IM** RISC-V SoC emulator written in Rust, implementing the custom **THEJAS32** processor with a rich set of on-chip peripherals and a GDB stub for hardware debugging.

## Features

### CPU Core
- **RV32IM** base integer instruction set (32-bit)
- **RV32C** compressed instruction support — 16-bit instructions decoded to 32-bit via `c_decode.rs`
- **RV32A** atomic extension — LR.W, SC.W, AMOADD.W, AMOSWAP.W, AMOXOR.W, AMOAND.W, AMOOR.W, AMOMIN.W, AMOMAX.W, AMOMINU.W, AMOMAXU.W
- Full CSR model with machine-mode traps: `mstatus`, `mie`, `mtvec`, `mepc`, `mcause`, `mtval`, `mip`, `mcycle`/`mcycleh`
- **PMP** (Physical Memory Protection) — 16 granularity entries with TOR, NA4, NAPOT addressing modes and lock bits
- Variable-length instruction fetch (16-bit compressed or 32-bit standard)
- Trap handling for interrupts, exceptions (access faults, ECALL, EBREAK), and system returns (MRET, WFI)

### Peripherals
| Peripheral | Base Address | Size     | Description |
|------------|-------------|----------|-------------|
| **CLINT**  | `0x0200_0000` | 64 KB   | Core Local Interruptor — 64-bit `mtime` counter, `mtimecmp` compare, `msip` software interrupt |
| **PLIC**   | `0x0C00_0000` | 4 MB    | Platform-Level Interrupt Controller — 32 sources with 8-level priorities, message-signaled mode |
| **UART**   | `0x1000_0000` | 4 KB    | 16550-style UART with crossterm raw terminal I/O — keyboard input, stdout, Ctrl+C exit |
| **GPIO**   | `0x1001_0000` | 4 KB    | GPIO — data in/out registers, direction control, atomic set/clear |
| **Timer**  | `0x1002_0000` | 4 KB    | Programmable timer — prescaler divider, auto-reload, match interrupt, enable control |
| **SPI**    | `0x1003_0000` | 4 KB    | SPI controller — loopback mode, virtual flash mock with JEDEC ID query (`0x9F` → `0xEF`) |
| **I2S**    | `0x1004_0000` | 4 KB    | I²S audio interface — stereo left/right sample channels, sample counter |
| **I2C**    | `0x1005_0000` | 4 KB    | I²C controller — virtual 24C02 EEPROM (256 bytes) at address `0x50`, start/stop/ACK support |
| **DMA**    | `0x1006_0000` | 4 KB    | DMA engine — up to 16 bytes/cycle, source/destination increment, interrupt on completion |
| **WDT**    | `0x1007_0000` | 4 KB    | Watchdog Timer — configurable timeout, system reset or interrupt on expire, feed with magic keys `0x5555_AAAA` / `0x76` |
| **QSPI**   | `0x4000_0000` | 16 MB   | Quad SPI flash window — execute-in-place (XIP), defaults to `0xFF` erased state |
| **SRAM**   | `0x8000_0000` | 512 KB  | Main system RAM with BSS zero-fill on ELF load |

## Memory Map

```
Address Range            Size        Region
────────────────────────────────────────────────────
0x0000_0000 – 0x3FFF_FFFF  — Unmapped / Read-only (returns 0)
0x4000_0000 – 0x40FF_FFFF  — QSPI Flash (16 MB XIP window)
0x8000_0000 – 0x8007_FFFF  — SRAM (512 KB)
────────────────────────────────────────────────────
0x0200_0000 – 0x0200_FFFF  — CLINT (Core Local Interruptor)
0x0C00_0000 – 0x0C3F_FFFF  — PLIC (Platform-Level Interrupt Controller)
0x1000_0000 – 0x1000_FFFF  — UART
0x1001_0000 – 0x1001_FFFF  — GPIO
0x1002_0000 – 0x1002_FFFF  — Timer
0x1003_0000 – 0x1003_FFFF  — SPI
0x1004_0000 – 0x1004_FFFF  — I2S
0x1005_0000 – 0x1005_FFFF  — I2C
0x1006_0000 – 0x1006_FFFF  — DMA
0x1007_0000 – 0x1007_FFFF  — WDT
```

## Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) ≥ **1.85** (edition 2024 requires this version)

```bash
git clone <repo-url>
cd Thejas32rs
cargo build --release
```

### Dependencies
| Crate | Version | Purpose |
|-------|---------|---------|
| `goblin` | 0.8 | ELF binary parsing (`Elf::parse`) |
| `crossterm` | 0.28 | Raw terminal mode, keyboard event polling for UART I/O |
| `clap` | 4.5 | CLI argument parsing with derive API |

## Usage

```bash
# Run a bare-metal ELF binary
./target/release/Thejas32rs <path-to-elf>

# With instruction trace disassembly and register state
./target/release/Thejas32rs <path-to-elf> --trace --dump-regs

# Limit execution to 10 million cycles
./target/release/Thejas32rs <path-to-elf> --max-cycles 10000000

# Enable GDB debug server on TCP port 1234
./target/release/Thejas32rs <path-to-elf> --gdb-port 1234
```

### CLI Options

| Flag | Short | Description |
|------|-------|-------------|
| `--trace` | `-t` | Print instruction trace: PC, raw encoding, mnemonic, rd/rs1/rs2 |
| `--dump-regs` | `-d` | Dump all 32 GPRs and CSR state (`mstatus`, `mepc`, `mcause`) on exit/crash |
| `--max-cycles` | `-c` | Maximum execution cycles (0 = unlimited, default: 0) |
| `--gdb-port` | | Enable GDB server on the specified TCP port |

### Execution Model

1. **Load** — ELF binary parsed with `goblin`; LOAD segments mapped to Flash (`0x4000_0000`) or SRAM (`0x8000_0000`), BSS zero-filled
2. **Reset** — CPU starts at ELF entry point, `x2` (sp) initialized to `0x8007_0000`, all other regs zeroed, priv mode set to Machine (3)
3. **Step loop** — each cycle: tick peripherals (WDT, UART, Timer, DMA, CLINT) → check interrupts → fetch (16/32-bit) → PMP access check → decode → execute → advance PC
4. **Interrupts** — checked each cycle: CLINT timer sets MIP bit 7 (MTIP), PLIC external sets MIP bit 11 (MEIP). External interrupts have higher priority than timer interrupts
5. **Trap** — on interrupt/exception: `mepc` ← current PC, `mcause` ← cause, `MPIE` ← `MIE`, `MIE` ← 0, PC ← `mtvec` base (direct) or `mtvec + cause × 4` (vectored)
6. **Exit** — clean shutdown when PC jumps to 0, or on WDT system reset (PC → `0x8000_0000`), max cycles reached, or GDB detach

### GDB Debugging

```bash
# Terminal 1: Start emulator with GDB server
./target/release/Thejas32rs hello.elf --gdb-port 1234

# Terminal 2: Connect with GDB
riscv64-unknown-elf-gdb hello.elf
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

Use the included `.gdbinit` for convenience:
```
target remote :1234
set confirm off
set pagination off
break main
continue
```

**Supported GDB packets:**

| Packet | Direction | Description |
|--------|-----------|-------------|
| `qSupported` | ← | Feature negotiation (`PacketSize=1000;hwbreak+;vContSupported-`) |
| `?` | ← | Return `S05` (TRAP / SIGTRAP) on stop |
| `g` | ← | Read all 33 registers (x0–x31 + PC) in little-endian hex |
| `G` | → | Write all 33 registers |
| `p<reg>` | ← | Read single register by index |
| `P<reg>=<val>` | → | Write single register |
| `m<addr>,<len>` | ← | Read `<len>` bytes at `<addr>` |
| `M<addr>,<len>:<data>` | → | Write `<len>` bytes at `<addr>` |
| `Z0,<addr>,4` | → | Insert hardware breakpoint at `<addr>` |
| `z0,<addr>,4` | → | Remove breakpoint |
| `s` | → | Single step |
| `c` | → | Continue execution |
| `D` / `k` | → | Detach / Kill |

## Peripheral Reference

### CLINT — Core Local Interruptor (`0x0200_0000`)

64-bit machine timer with software interrupt support.

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x0000` | `msip` | 32-bit | R/W | Machine Software Interrupt Pending (bit 0) |
| `0x4000` | `mtimecmp` | 32-bit | R/W | Timer compare low 32 bits |
| `0x4004` | `mtimecmph` | 32-bit | R/W | Timer compare high 32 bits |
| `0xBFF8` | `mtime` | 32-bit | R/W | Timer counter low 32 bits |
| `0xBFFC` | `mtimeh` | 32-bit | R/W | Timer counter high 32 bits |

The `mtime` register increments by 1 every SoC clock cycle. When `mtime >= mtimecmp`, the timer interrupt pending bit is set.

### PLIC — Platform-Level Interrupt Controller (`0x0C00_0000`)

Message-signaled interrupt controller with 32 sources and 8 priority levels.

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x000004–0x00007C` | `priority[N]` | 32-bit | R/W | Priority for interrupt source N (1–31), 8 levels |
| `0x001000` | `pending` | 32-bit | R/W | Pending bitmask (write to clear) |
| `0x002000` | `enabled` | 32-bit | R/W | Enable bitmask for Hart 0 |
| `0x200000` | `threshold` | 32-bit | R/W | Priority threshold — only sources > threshold are signaled |
| `0x200004` | `claimcomplete` | 32-bit | R/W | Read: claims highest-priority interrupt. Write: signal completion |

**Interrupt source mapping:**

| Source ID | Peripheral | Trigger Condition |
|-----------|------------|-------------------|
| 1 | UART | RX data ready (bit 0 of UART LSR) |
| 2 | Timer | Timer match interrupt |
| 3 | DMA | DMA transfer complete |
| 4 | WDT | Watchdog timeout |

### UART — 16550-style UART (`0x1000_0000`)

Full-duplex UART with crossterm terminal integration. Keyboard events (press only) are polled every 10 ms into the receive buffer. Control+C exits raw mode and terminates the emulator.

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `THR / RHR` | 8-bit | R/W | Transmit Holding Register / Receive Buffer |
| `0x05` | `LSR` | 8-bit | RO | Line Status: bit 0 = data ready, bits 5–6 = TX ready (`0x60`) |

### GPIO — General-Purpose I/O (`0x1001_0000`)

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `data_in` | 32-bit | RO | Combined input: `(data_in & ~direction) \| (data_out & direction)` |
| `0x04` | `data_out` | 32-bit | R/W | Output data |
| `0x08` | `direction` | 32-bit | R/W | 1 = output, 0 = input |
| `0x0C` | `set` | 32-bit | W | Atomic bit set on `data_out` |
| `0x10` | `clear` | 32-bit | W | Atomic bit clear on `data_out` |

### Timer (`0x1002_0000`)

Programmable interval timer with prescaler and auto-reload.

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `ctrl` | 32-bit | R/W | bit 0: enable, bit 1: irq_en, bit 2: auto_reload |
| `0x04` | `count` | 32-bit | R/W | Current counter value |
| `0x08` | `reload` | 32-bit | R/W | Compare/reload value (default: `0xFFFF_FFFF`) |
| `0x0C` | `presc` | 32-bit | R/W | Prescaler divider |
| `0x10` | `status` | 32-bit | W1C | bit 0: match interrupt flag (write 1 to clear) |

### SPI (`0x1003_0000`)

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `cr` | 32-bit | R/W | bit 0: enable, bit 1: loopback mode |
| `0x04` | `sr` | 32-bit | RO | bit 0: TX ready, bit 1: RX ready |
| `0x08` | `data` | 32-bit | R/W | TX/RX data register |

**Loopback mode** (`cr` bit 1): transmitted byte is immediately returned in `data`.  
**Virtual flash mode**: byte `0x9F` (JEDEC ID) returns `0xEF` (Winbond); other bytes return `byte + 1`.

### I²S (`0x1004_0000`)

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `cr` | 32-bit | R/W | bit 0: enable, bit 1: 16/32-bit sample width |
| `0x04` | `sr` | 32-bit | RO | bit 0: always ready (1) |
| `0x08` | `left_tx` | 32-bit | R/W | Left audio channel sample |
| `0x0C` | `right_tx` | 32-bit | R/W | Right audio channel sample |

### I²C (`0x1005_0000`)

Virtual 24C02 EEPROM (256 bytes) at address `0x50`. Supports START condition, read/write operations, ACK/NACK status.

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `cr` | 32-bit | R/W | bit 2: START, bit 3: read direction |
| `0x04` | `sr` | 32-bit | R/W | bit 2: ACK, bit 1: RX ready |
| `0x08` | `addr` | 32-bit | R/W | 7-bit device address |
| `0x0C` | `data` | 32-bit | R/W | Data register |

### DMA (`0x1006_0000`)

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `src` | 32-bit | R/W | Source address |
| `0x04` | `dst` | 32-bit | R/W | Destination address |
| `0x08` | `len` | 32-bit | R/W | Transfer length in bytes |
| `0x0C` | `ctrl` | 32-bit | R/W | bit 0: START, bit 1: done, bit 2: src_inc, bit 3: dst_inc, bit 2: irq_en |
| `0x10` | `status` | 32-bit | W1C | bit 0: BUSY, bit 1: DONE (write 1 to clear) |

Transfers move up to 16 bytes per cycle. On completion, `status` sets DONE and triggers PLIC source 3 if IRQ enabled.

### WDT — Watchdog Timer (`0x1007_0000`)

| Offset | Register | Size | R/W | Description |
|--------|----------|------|-----|-------------|
| `0x00` | `ctrl` | 32-bit | R/W | bit 0: enable, bit 1: reset enable |
| `0x04` | `count` | 32-bit | R/W | Current countdown value (default: `0xFFFF_FFFF`) |
| `0x08` | `reload` | 32-bit | R/W | Reload value on timeout (default: `0xFFFF_FFFF`) |
| `0x0C` | `feed` | 32-bit | W | Feed key: `0x5555_AAAA` or `0x76` to reset countdown |
| `0x10` | `status` | 32-bit | W1C | bit 0: timeout flag (write 1 to clear) |

If `ctrl` bit 0 is set, `count` decrements each cycle. On timeout: if `ctrl` bit 1 is set, the CPU resets to `0x8000_0000`; otherwise a WDT interrupt is asserted on PLIC line 4.

### QSPI Flash (`0x4000_0000`)

16 MB flash window. Defaults to `0xFF` (erased). Sections from loaded ELF files are written via `load_bytes`. Write operations are ignored (read-only window).

## CSR Reference

Machine-mode Control and Status Registers, starting in Machine mode (priv mode = 3).

| CSR Address | Name | Description |
|-------------|------|-------------|
| `0x300` | `mstatus` | Machine status: MIE (bit 3), MPIE (bit 7), MPP (bits 11–12) |
| `0x301` | `misa` | ISA extension flags: MXL=1, A, C, I, M |
| `0x304` | `mie` | Machine interrupt enable: MTIE (bit 7), MEIE (bit 11) |
| `0x305` | `mtvec` | Machine trap vector: base address + mode (0=direct, 1=vectored) |
| `0x341` | `mepc` | Machine exception program counter |
| `0x342` | `mcause` | Machine trap cause (interrupts have bit 31 set) |
| `0x343` | `mtval` | Machine trap value (faulting address) |
| `0x344` | `mip` | Machine interrupt pending: MTIP (bit 7), MEIP (bit 11) |
| `0xB00` | `mcycle` | Cycle counter low 32 bits |
| `0xC80` | `mcycleh` | Cycle counter high 32 bits |
| `0x3A0–0x3A3` | `pmpcfg0–3` | PMP configuration (R/W/X, mode, lock) |
| `0x3B0–0x3BF` | `pmpaddr0–15` | PMP address registers |

### Interrupt Cause Codes

| Cause | Value | Type |
|-------|-------|------|
| Instruction access fault | `1` | Exception |
| Load access fault | `5` | Exception |
| Store access fault | `7` | Exception |
| ECALL | `11` | Exception |
| EBREAK | `3` | Exception |
| Machine timer interrupt | `0x8000_0007` | Interrupt |
| Machine external interrupt | `0x8000_000B` | Interrupt |

## Instruction Set

The CPU implements the full RV32IM + RV32C + RV32A instruction set:

### RV32I Base
| Type | Instructions |
|------|-------------|
| ALU | `ADD`, `SUB`, `SLL`, `SLT`, `SLTU`, `XOR`, `SRL`, `SRA`, `OR`, `AND` |
| Immediate | `ADDI`, `SLTI`, `SLTIU`, `XORI`, `ORI`, `ANDI`, `SLLI`, `SRLI`, `SRAI` |
| Load | `LB`, `LH`, `LW`, `LBU`, `LHU` |
| Store | `SB`, `SH`, `SW` |
| Branch | `BEQ`, `BNE`, `BLT`, `BGE`, `BLTU`, `BGEU` |
| Jump | `JAL`, `JALR` |
| U-type | `LUI`, `AUIPC` |
| System | `ECALL`, `EBREAK`, `MRET`, `WFI`, CSRRW/CSRRRS/CSRRRC/CSRRWI/CSRRSI/CSRRCI |

### RV32M Extension
`MUL`, `MULH`, `MULHSU`, `MULHU`, `DIV`, `DIVU`, `REM`, `REMU`

### RV32C Compressed
`C.ADDI4SPN`, `C.ADDI`, `C.LI`, `C.ADDI16SP`, `C.LUI`, `C.SLLI`, `C.SRLI`, `C.SRAI`, `C.ANDI`, `C.SUB`, `C.XOR`, `C.OR`, `C.AND`, `C.JAL`, `C.J`, `C.BEQZ`, `C.BNEZ`, `C.JALR`, `C.JR`, `C.MV`, `C.ADD`, `C.EBREAK`, `C.LWSP`, `C.SWSP`, `C.LW`, `C.SW`

### RV32A Atomic
`LR.W`, `SC.W`, `AMOSWAP.W`, `AMOADD.W`, `AMOXOR.W`, `AMOAND.W`, `AMOOR.W`, `AMOMIN.W`, `AMOMAX.W`, `AMOMINU.W`, `AMOMAXU.W`

## Project Structure

```
Thejas32rs/
├── Cargo.toml              # Rust project manifest (edition 2024)
├── .gdbinit                # Default GDB session configuration
├── .gitignore              # Cargo build artifacts
├── src/
│   ├── main.rs             # CLI entry point, execution loop, raw mode guard
│   ├── bus.rs              # Memory-mapped bus: RAM, Flash, all peripherals, ELF loader
│   ├── cpu/
│   │   ├── mod.rs          # CPU core: fetch/decode/execute, interrupts, traps, step()
│   │   ├── decode.rs       # Instruction decode: opcode, funct fields, immediates, mnemonics
│   │   ├── c_decode.rs     # RV32C decompressor: 16-bit → 32-bit instruction mapping
│   │   ├── csr.rs          # CSR file: mstatus/mie/mtvec/mepc/mcause/mtval/mip/mcycle + PMP
│   │   ├── pmp.rs          # Physical Memory Protection: 16 entries, TOR/NA4/NAPOT modes
│   │   └── execute.rs      # (placeholder)
│   ├── clint.rs            # Core Local Interruptor: 64-bit timer + msip
│   ├── plic.rs             # Platform-Level Interrupt Controller: 32 sources
│   ├── uart.rs             # UART with crossterm terminal I/O + background polling thread
│   ├── gpio.rs             # GPIO with data in/out, direction, set/clear
│   ├── timer.rs            # Programmable timer with prescaler and auto-reload
│   ├── spi.rs              # SPI with loopback mode and virtual flash mock
│   ├── i2s.rs              # I²S audio interface
│   ├── i2c.rs              # I²C with virtual 24C02 EEPROM
│   ├── dma.rs              # DMA engine with source/destination increment
│   ├── wdt.rs              # Watchdog timer with reset/interrupt modes
│   ├── qspi.rs             # QSPI flash window (16 MB)
│   └── gdb.rs              # GDB stub: TCP server, packet protocol, breakpoints
├── programs/
│   └── hello.c             # Bare-metal C example: UART "Hello World"
└── README.md               # This file
```

## Building a Program

The `programs/hello.c` example demonstrates bare-metal C programming:

```c
#define UART_THR (*((volatile unsigned char *)0x10000000))

void uart_puts(const char *s) {
    while (*s) {
        UART_THR = *s++;
    }
}

void _start() {
    uart_puts("Hello from bare-metal C on THEJAS32 Emulator!\n");
    while (1);
}
```

Compile with a RISC-V cross-compiler:

```bash
# Using riscv64-unknown-elf-gcc
riscv64-unknown-elf-gcc -march=rv32im -mabi=ilp32 -nostdlib -ffreestanding \
    -T linker.ld programs/hello.c -o hello.elf

# Run with trace
./target/release/Thejas32rs hello.elf --trace --dump-regs
```

## Development

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run all checks
cargo check
cargo clippy

# Run with GDB
./target/release/Thejas32rs programs/hello.elf --gdb-port 1234
```

## Credits

Built by [Dhruv Goel](https://github.com/dhruvgoel3829) — part of the [Thejas32](https://github.com/dhruvgoel3829/Thejas32) project.

### Changelog (by commit)
- **ELF loading** — binary execution support
- **CLINT** — 64-bit hardware timer & preemptive timer interrupts
- **UART** — full-duplex MMIO UART with PLIC interrupt line
- **PLIC** — platform-level interrupt controller
- **SPI, I2S, GPIO, Timer** — peripheral support
- **I2C** — I²C bus with virtual EEPROM
- **Atomic instructions** — RV32A extension (LR.W, SC.W, AMO*)
- **Compressed instructions** — RV32C 16-bit instruction decode
- **DMA** — direct memory access engine
- **GDB** — TCP GDB stub with breakpoints and register/memory access
- **Scheduler** — interrupt-aware instruction scheduling
- **PMP** — physical memory protection (16 entries)
- **WDT** — hardware watchdog timer with reset capability
- **QSPI** — 16 MB flash execute-in-place window
- **Tests** — test suite
