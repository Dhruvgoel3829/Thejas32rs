thejas32/
├── Cargo.toml
└── src/
    ├── main.rs          # CLI entry point, loading binary/ELF
    ├── cpu/
    │   ├── mod.rs       # Struct Cpu: registers, PC, CSRs, step()
    │   ├── decode.rs    # Decodes raw u32 into typed Instruction enum/struct
    │   └── execute.rs   # Instruction execution logic
    ├── bus.rs           # Interconnect / Memory Bus (routes addresses)
    └── devices/
        ├── mod.rs       # Trait Device: read_byte/u32, write_byte/u32
        ├── ram.rs       # 256KB SRAM buffer
        ├── uart.rs      # UART transmitter mapped to stdout
        └── plic.rs      # Platform-Level Interrupt Controller