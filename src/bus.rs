// src/bus.rs
use crate::adc::{Adc, ADC_BASE};
use crate::clint::{Clint, CLINT_BASE};
use crate::dma::{Dma, DMA_BASE};
use crate::gpio::{Gpio, GPIO_BASE};
use crate::i2c::{I2c, I2C_BASE};
use crate::i2s::{I2s, I2S_BASE};
use crate::plic::{Plic, PLIC_BASE};
use crate::pwm::{Pwm, PWM_BASE};
use crate::qspi::{QspiFlash, FLASH_BASE};
use crate::spi::{Spi, SPI_BASE};
use crate::timer::{Timer, TIMER_BASE};
use crate::uart::{Uart, UART0_BASE, UART1_BASE, UART2_BASE};
use crate::wdt::{Wdt, WDT_BASE};

use goblin::elf::Elf;
use std::fs;

pub const RAM_BASE: u32 = 0x8000_0000;
pub const RAM_SIZE: usize = 512 * 1024; // 512 KB

pub struct Bus {
    pub ram: Vec<u8>,
    pub flash: QspiFlash,
    pub uart0: Uart,
    pub uart1: Uart,
    pub uart2: Uart,
    pub clint: Clint,
    pub plic: Plic,
    pub gpio: Gpio,
    pub timer: Timer,
    pub spi: Spi,
    pub i2s: I2s,
    pub i2c: I2c,
    pub adc: Adc,
    pub pwm: Pwm,
    pub dma: Dma,
    pub wdt: Wdt,
    pub strict_mem: bool,
    pub test_finished_reg: Option<u32>,
}

impl Bus {
    pub fn new(enable_ipc: bool, flash_file: Option<String>, strict_mem: bool, test_finished_reg: Option<u32>) -> Self {
        let ipc = enable_ipc;
        Self {
            ram: vec![0; RAM_SIZE],
            flash: QspiFlash::with_backing_file(flash_file),
            uart0: Uart::new(if enable_ipc { Some("/tmp/thejas_uart0.sock") } else { None }),
            uart1: Uart::new(if enable_ipc { Some("/tmp/thejas_uart1.sock") } else { None }),
            uart2: Uart::new(if enable_ipc { Some("/tmp/thejas_uart2.sock") } else { None }),
            clint: Clint::new(),
            plic: Plic::new(),
            gpio: Gpio::new(enable_ipc),
            timer: Timer::new(),
            spi: Spi::new(enable_ipc),
            i2s: I2s::new(),
            i2c: I2c::new(enable_ipc),
            adc: Adc::new(enable_ipc),
            pwm: Pwm::new(enable_ipc),
            dma: Dma::new(),
            wdt: Wdt::new(),
            strict_mem,
            test_finished_reg,
        }
    }

    pub fn load_elf(&mut self, path: &str) -> u32 {
        let buffer = fs::read(path).expect("Failed to read ELF file");
        let elf = Elf::parse(&buffer).expect("Invalid ELF file format");

        for header in &elf.program_headers {
            if header.p_type == goblin::elf::program_header::PT_LOAD {
                let vaddr = header.p_vaddr as u32;
                let offset = header.p_offset as usize;
                let filesz = header.p_filesz as usize;
                let memsz = header.p_memsz as usize;
                let file_data = &buffer[offset..offset + filesz];

                let seg_start = vaddr;
                let seg_end = vaddr + memsz as u32 - 1;
                let ram_start = RAM_BASE;
                let ram_end = RAM_BASE + (RAM_SIZE as u32) - 1;

                if seg_end >= ram_start && seg_start <= ram_end {
                    let load_start = seg_start.max(ram_start);
                    let load_end = seg_end.min(ram_end);
                    let load_offset = (load_start - seg_start) as usize;
                    let load_len = (load_end - load_start + 1) as usize;
                    let ram_offset = (load_start - RAM_BASE) as usize;

                    if load_len > 0 && filesz > 0 {
                        let copy_len = load_len.min(filesz - load_offset);
                        self.ram[ram_offset..ram_offset + copy_len].copy_from_slice(&file_data[load_offset..load_offset + copy_len]);
                    }

                    let bss_end = (ram_offset + memsz).min(self.ram.len());
                    if ram_offset + load_len < bss_end {
                        self.ram[ram_offset + load_len..bss_end].fill(0);
                    }
                } else if (0x4000_0000..=0x40FF_FFFF).contains(&vaddr) {
                    let flash_offset = vaddr - FLASH_BASE;
                    self.flash.load_bytes(flash_offset, file_data);
                }
            }
        }

        elf.entry as u32
    }

    pub fn tick_dma(&mut self, chunk_size: usize) -> bool {
        if !self.dma.is_active() {
            return false;
        }

        let to_transfer = std::cmp::min(self.dma.bytes_remaining as usize, chunk_size);
        let src_inc = (self.dma.ctrl & 0x04) != 0;
        let dst_inc = (self.dma.ctrl & 0x08) != 0;

        for _ in 0..to_transfer {
            let byte = self.read_u8(self.dma.current_src);
            self.write_u8(self.dma.current_dst, byte);

            if src_inc {
                self.dma.current_src = self.dma.current_src.wrapping_add(1);
            }
            if dst_inc {
                self.dma.current_dst = self.dma.current_dst.wrapping_add(1);
            }

            self.dma.bytes_remaining -= 1;
        }

        if self.dma.bytes_remaining == 0 {
            self.dma.status &= !0x01; // Clear BUSY
            self.dma.status |= 0x02;  // Set DONE
            self.dma.ctrl &= !0x01;   // Clear START
            let irq_en = (self.dma.ctrl & 0x02) != 0;
            return irq_en;
        }

        false
    }

    pub fn read_u8(&mut self, addr: u32) -> u8 {
        // Check for test-finished register read
        if let Some(reg) = self.test_finished_reg {
            if addr == reg {
                // Reading the test-finished register returns 0
                return 0;
            }
        }

        let result = match addr {
            // Flash XIP Memory Window (0x4000_0000 - 0x40FF_FFFF)
            0x4000_0000..=0x40FF_FFFF => self.flash.read_u8(addr - FLASH_BASE),

            // SRAM (0x8000_0000 - 0x8007_FFFF)
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                self.ram[offset]
            }

            // Peripherals
            0x0200_0000..=0x0200_FFFF => self.clint.read_u8(addr - CLINT_BASE),
            0x0C00_0000..=0x0C3F_FFFF => {
                let offset = addr - PLIC_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.plic.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            // UART0 (Telemetry) at 0x1000_0000
            0x1000_0000..=0x1000_0FFF => self.uart0.read_u8(addr - UART0_BASE),
            // UART1 (GNSS) at 0x1000_1000
            0x1000_1000..=0x1000_1FFF => self.uart1.read_u8(addr - UART1_BASE),
            // UART2 (RC) at 0x1000_2000
            0x1000_2000..=0x1000_2FFF => self.uart2.read_u8(addr - UART2_BASE),
            0x1001_0000..=0x1001_0FFF => {
                let offset = addr - GPIO_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.gpio.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1002_0000..=0x1002_0FFF => {
                let offset = addr - TIMER_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.timer.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1003_0000..=0x1003_0FFF => {
                let offset = addr - SPI_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.spi.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1004_0000..=0x1004_0FFF => {
                let offset = addr - I2S_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.i2s.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1005_0000..=0x1005_0FFF => {
                let offset = addr - I2C_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.i2c.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1008_0000..=0x1008_0FFF => {
                let offset = addr - ADC_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.adc.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1009_0000..=0x1009_0FFF => {
                let offset = addr - PWM_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.pwm.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1006_0000..=0x1006_0FFF => {
                let offset = addr - DMA_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.dma.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            0x1007_0000..=0x1007_0FFF => {
                let offset = addr - WDT_BASE;
                let shift = (offset & 0x3) * 8;
                ((self.wdt.read_u32(offset & !0x3) >> shift) & 0xFF) as u8
            }
            _ => {
                // Strict memory mode: fault on unmapped access
                if self.strict_mem {
                    panic!("Strict memory mode: read from unmapped address 0x{:08x}", addr);
                }
                0
            },
        };
        result
    }

    pub fn write_u8(&mut self, addr: u32, val: u8) {
        // Check for test-finished register write
        if let Some(reg) = self.test_finished_reg {
            if addr == reg {
                // Writing to test-finished register sets exit code
                std::process::exit(val as i32);
            }
        }

        match addr {
            // Flash (QSPI) - now writable for byte programming
            0x4000_0000..=0x40FF_FFFF => {
                self.flash.write_u8(addr - FLASH_BASE, val);
            }

            // SRAM
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                self.ram[offset] = val;
            }

            // Peripherals
            0x0200_0000..=0x0200_FFFF => self.clint.write_u8(addr - CLINT_BASE, val),
            0x0C00_0000..=0x0C3F_FFFF => {
                let offset = addr - PLIC_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.plic.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.plic.write_u32(word_offset, current);
            }
            // UART0 (Telemetry) at 0x1000_0000
            0x1000_0000..=0x1000_0FFF => self.uart0.write_u8(addr - UART0_BASE, val),
            // UART1 (GNSS) at 0x1000_1000
            0x1000_1000..=0x1000_1FFF => self.uart1.write_u8(addr - UART1_BASE, val),
            // UART2 (RC) at 0x1000_2000
            0x1000_2000..=0x1000_2FFF => self.uart2.write_u8(addr - UART2_BASE, val),
            0x1001_0000..=0x1001_0FFF => {
                let offset = addr - GPIO_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.gpio.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.gpio.write_u32(word_offset, current);
            }
            0x1002_0000..=0x1002_0FFF => {
                let offset = addr - TIMER_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.timer.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.timer.write_u32(word_offset, current);
            }
            0x1003_0000..=0x1003_0FFF => {
                let offset = addr - SPI_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.spi.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.spi.write_u32(word_offset, current);
            }
            0x1004_0000..=0x1004_0FFF => {
                let offset = addr - I2S_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.i2s.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.i2s.write_u32(word_offset, current);
            }
            0x1005_0000..=0x1005_0FFF => {
                let offset = addr - I2C_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.i2c.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.i2c.write_u32(word_offset, current);
            }
            0x1008_0000..=0x1008_0FFF => {
                let offset = addr - ADC_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.adc.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.adc.write_u32(word_offset, current);
            }
            0x1009_0000..=0x1009_0FFF => {
                let offset = addr - PWM_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.pwm.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.pwm.write_u32(word_offset, current);
            }
            0x1006_0000..=0x1006_0FFF => {
                let offset = addr - DMA_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.dma.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.dma.write_u32(word_offset, current);
            }
            0x1007_0000..=0x1007_0FFF => {
                let offset = addr - WDT_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.wdt.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.wdt.write_u32(word_offset, current);
            }
            _ => {
                // Strict memory mode: fault on unmapped access
                if self.strict_mem {
                    panic!("Strict memory mode: write to unmapped address 0x{:08x}", addr);
                }
            }
        }
    }

    pub fn read_u16(&mut self, addr: u32) -> u16 {
        match addr {
            0x4000_0000..=0x40FF_FFFF => self.flash.read_u16(addr - FLASH_BASE),
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                let b0 = self.ram[offset] as u16;
                let b1 = self.ram[offset + 1] as u16;
                b0 | (b1 << 8)
            }
            _ => (self.read_u8(addr) as u16) | ((self.read_u8(addr.wrapping_add(1)) as u16) << 8),
        }
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) {
        match addr {
            0x4000_0000..=0x40FF_FFFF => {} // Read-only
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                let bytes = val.to_le_bytes();
                self.ram[offset] = bytes[0];
                self.ram[offset + 1] = bytes[1];
            }
            _ => {
                self.write_u8(addr, (val & 0xFF) as u8);
                self.write_u8(addr.wrapping_add(1), ((val >> 8) & 0xFF) as u8);
            }
        }
    }

    pub fn read_u32(&mut self, addr: u32) -> u32 {
        match addr {
            0x4000_0000..=0x40FF_FFFF => self.flash.read_u32(addr - FLASH_BASE),
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                let b0 = self.ram[offset] as u32;
                let b1 = self.ram[offset + 1] as u32;
                let b2 = self.ram[offset + 2] as u32;
                let b3 = self.ram[offset + 3] as u32;
                b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            }
            0x0200_0000..=0x0200_FFFF => self.clint.read_u32(addr - CLINT_BASE),
            0x0C00_0000..=0x0C3F_FFFF => self.plic.read_u32(addr - PLIC_BASE),
            // UART0 (Telemetry) at 0x1000_0000
            0x1000_0000..=0x1000_0FFF => self.uart0.read_u8(addr - UART0_BASE) as u32,
            // UART1 (GNSS) at 0x1000_1000
            0x1000_1000..=0x1000_1FFF => self.uart1.read_u8(addr - UART1_BASE) as u32,
            // UART2 (RC) at 0x1000_2000
            0x1000_2000..=0x1000_2FFF => self.uart2.read_u8(addr - UART2_BASE) as u32,
            0x1001_0000..=0x1001_0FFF => self.gpio.read_u32(addr - GPIO_BASE),
            0x1002_0000..=0x1002_0FFF => self.timer.read_u32(addr - TIMER_BASE),
            0x1003_0000..=0x1003_0FFF => self.spi.read_u32(addr - SPI_BASE),
            0x1004_0000..=0x1004_0FFF => self.i2s.read_u32(addr - I2S_BASE),
            0x1005_0000..=0x1005_0FFF => self.i2c.read_u32(addr - I2C_BASE),
            0x1008_0000..=0x1008_0FFF => self.adc.read_u32(addr - ADC_BASE),
            0x1009_0000..=0x1009_0FFF => self.pwm.read_u32(addr - PWM_BASE),
            0x1006_0000..=0x1006_0FFF => self.dma.read_u32(addr - DMA_BASE),
            0x1007_0000..=0x1007_0FFF => self.wdt.read_u32(addr - WDT_BASE),
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        match addr {
            // Flash (QSPI) - writable for word programming
            0x4000_0000..=0x40FF_FFFF => {
                let bytes = val.to_le_bytes();
                self.flash.write_bytes(addr - FLASH_BASE, &bytes);
            }
            0x8000_0000..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                let bytes = val.to_le_bytes();
                self.ram[offset..offset + 4].copy_from_slice(&bytes);
            }
            0x0200_0000..=0x0200_FFFF => self.clint.write_u32(addr - CLINT_BASE, val),
            0x0C00_0000..=0x0C3F_FFFF => self.plic.write_u32(addr - PLIC_BASE, val),
            // UART0 (Telemetry) at 0x1000_0000
            0x1000_0000..=0x1000_0FFF => self.uart0.write_u8(addr - UART0_BASE, val as u8),
            // UART1 (GNSS) at 0x1000_1000
            0x1000_1000..=0x1000_1FFF => self.uart1.write_u8(addr - UART1_BASE, val as u8),
            // UART2 (RC) at 0x1000_2000
            0x1000_2000..=0x1000_2FFF => self.uart2.write_u8(addr - UART2_BASE, val as u8),
            0x1001_0000..=0x1001_0FFF => self.gpio.write_u32(addr - GPIO_BASE, val),
            0x1002_0000..=0x1002_0FFF => self.timer.write_u32(addr - TIMER_BASE, val),
            0x1003_0000..=0x1003_0FFF => self.spi.write_u32(addr - SPI_BASE, val),
            0x1004_0000..=0x1004_0FFF => self.i2s.write_u32(addr - I2S_BASE, val),
            0x1005_0000..=0x1005_0FFF => self.i2c.write_u32(addr - I2C_BASE, val),
            0x1008_0000..=0x1008_0FFF => self.adc.write_u32(addr - ADC_BASE, val),
            0x1009_0000..=0x1009_0FFF => self.pwm.write_u32(addr - PWM_BASE, val),
            0x1006_0000..=0x1006_0FFF => self.dma.write_u32(addr - DMA_BASE, val),
            0x1007_0000..=0x1007_0FFF => self.wdt.write_u32(addr - WDT_BASE, val),
            _ => {}
        }
    }
}
