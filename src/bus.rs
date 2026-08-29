use crate::clint::{CLINT_BASE, Clint};
use crate::dma::{DMA_BASE, Dma};
use crate::gpio::{GPIO_BASE, Gpio};
use crate::i2c::{I2C_BASE, I2c};
use crate::i2s::{I2S_BASE, I2s};
use crate::plic::{PLIC_BASE, Plic};
use crate::spi::{SPI_BASE, Spi};
use crate::timer::{TIMER_BASE, Timer};
use crate::uart::Uart;
use crate::wdt::{WDT_BASE, Wdt, WdtAction};
use goblin::elf::Elf;
use std::fs;

pub const RAM_BASE: u32 = 0x8000_0000;
pub const RAM_SIZE: usize = 512 * 1024;

pub struct Bus {
    pub ram: Vec<u8>,
    pub uart: Uart,
    pub clint: Clint,
    pub plic: Plic,
    pub gpio: Gpio,
    pub timer: Timer,
    pub spi: Spi,
    pub i2s: I2s,
    pub i2c: I2c,
    pub dma: Dma,
    pub wdt: Wdt,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: vec![0; RAM_SIZE],
            uart: Uart::new(),
            clint: Clint::new(),
            plic: Plic::new(),
            gpio: Gpio::new(),
            timer: Timer::new(),
            spi: Spi::new(),
            i2s: I2s::new(),
            i2c: I2c::new(),
            dma: Dma::new(),
            wdt: Wdt::new(),
        }
    }
    /// Tick the DMA engine: transfers up to `chunk_size` bytes per cycle
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

        // If transfer completed
        if self.dma.bytes_remaining == 0 {
            self.dma.status &= !0x01; // Clear BUSY
            self.dma.status |= 0x02; // Set DONE
            self.dma.ctrl &= !0x01; // Clear START bit

            let irq_en = (self.dma.ctrl & 0x02) != 0;
            return irq_en; // Returns true if PLIC interrupt should fire
        }

        false
    }

    pub fn load_elf(&mut self, path: &str) -> u32 {
        let buffer = fs::read(path).expect("Failed to read ELF file");
        let elf = Elf::parse(&buffer).expect("Failed to parse ELF");

        for ph in elf.program_headers.iter() {
            if ph.p_type == 1 {
                let vaddr = ph.p_vaddr as u32;
                let offset = ph.p_offset as usize;
                let filesz = ph.p_filesz as usize;
                let memsz = ph.p_memsz as usize;

                let segment_data = &buffer[offset..offset + filesz];

                for (i, &byte) in segment_data.iter().enumerate() {
                    self.write_u8(vaddr + i as u32, byte);
                }

                for i in filesz..memsz {
                    self.write_u8(vaddr + i as u32, 0);
                }
            }
        }

        elf.entry as u32
    }

    pub fn read_u8(&mut self, addr: u32) -> u8 {
        match addr {
            RAM_BASE..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                self.ram[offset]
            }
            0x1000_0000..=0x1000_00FF => self.uart.read_u8(addr - 0x1000_0000),
            0x0200_0000..=0x0200_FFFF => self.clint.read_u8(addr - CLINT_BASE),
            0x0C00_0000..=0x0C3F_FFFF => {
                let offset = addr - PLIC_BASE;
                let word_offset = offset & !0x3;
                let shift = (offset & 0x3) * 8;
                ((self.plic.read_u32(word_offset) >> shift) & 0xFF) as u8
            }
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
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, addr: u32, val: u8) {
        match addr {
            RAM_BASE..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                self.ram[offset] = val;
            }
            0x1000_0000..=0x1000_00FF => self.uart.write_u8(addr - 0x1000_0000, val),
            0x0200_0000..=0x0200_FFFF => self.clint.write_u8(addr - CLINT_BASE, val),
            0x0C00_0000..=0x0C3F_FFFF => {
                let offset = addr - PLIC_BASE;
                let word_offset = offset & !0x3;
                let mut current = self.plic.read_u32(word_offset);
                let shift = (offset & 0x3) * 8;
                current = (current & !(0xFF << shift)) | ((val as u32) << shift);
                self.plic.write_u32(word_offset, current);
            }
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
            _ => {}
        }
    }

    pub fn read_u16(&mut self, addr: u32) -> u16 {
        let b0 = self.read_u8(addr) as u16;
        let b1 = self.read_u8(addr + 1) as u16;
        b0 | (b1 << 8)
    }

    pub fn read_u32(&mut self, addr: u32) -> u32 {
        // MMIO peripherals only support 4-byte aligned word reads
        if (0x0C00_0000..=0x1005_FFFF).contains(&addr) && (addr % 4 == 0) {
            match addr {
                0x0C00_0000..=0x0C3F_FFFF => return self.plic.read_u32(addr - PLIC_BASE),
                0x1001_0000..=0x1001_0FFF => return self.gpio.read_u32(addr - GPIO_BASE),
                0x1002_0000..=0x1002_0FFF => return self.timer.read_u32(addr - TIMER_BASE),
                0x1003_0000..=0x1003_0FFF => return self.spi.read_u32(addr - SPI_BASE),
                0x1004_0000..=0x1004_0FFF => return self.i2s.read_u32(addr - I2S_BASE),
                0x1005_0000..=0x1005_0FFF => return self.i2c.read_u32(addr - I2C_BASE),
                0x1006_0000..=0x1006_0FFF => return self.dma.read_u32(addr - DMA_BASE),
                0x1007_0000..=0x1007_0FFF => return self.wdt.read_u32(addr - WDT_BASE),
                _ => {}
            }
        }

        // RAM reads: always byte-compose to seamlessly support 2-byte aligned 32-bit instructions
        let b0 = self.read_u8(addr) as u32;
        let b1 = self.read_u8(addr + 1) as u32;
        let b2 = self.read_u8(addr + 2) as u32;
        let b3 = self.read_u8(addr + 3) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) {
        self.write_u8(addr, val as u8);
        self.write_u8(addr + 1, (val >> 8) as u8);
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        if addr % 4 == 0 {
            match addr {
                0x0C00_0000..=0x0C3F_FFFF => {
                    self.plic.write_u32(addr - PLIC_BASE, val);
                    return;
                }
                0x1001_0000..=0x1001_0FFF => {
                    self.gpio.write_u32(addr - GPIO_BASE, val);
                    return;
                }
                0x1002_0000..=0x1002_0FFF => {
                    self.timer.write_u32(addr - TIMER_BASE, val);
                    return;
                }
                0x1003_0000..=0x1003_0FFF => {
                    self.spi.write_u32(addr - SPI_BASE, val);
                    return;
                }
                0x1004_0000..=0x1004_0FFF => {
                    self.i2s.write_u32(addr - I2S_BASE, val);
                    return;
                }
                0x1005_0000..=0x1005_0FFF => {
                    self.i2c.write_u32(addr - I2C_BASE, val);
                    return;
                }
                0x1006_0000..=0x1006_0FFF => {
                    self.dma.write_u32(addr - DMA_BASE, val);
                    return;
                }
                0x1007_0000..=0x1007_0FFF => {
                    self.wdt.write_u32(addr - WDT_BASE, val);
                    return;
                }
                _ => {}
            }
        }

        self.write_u8(addr, val as u8);
        self.write_u8(addr + 1, (val >> 8) as u8);
        self.write_u8(addr + 2, (val >> 16) as u8);
        self.write_u8(addr + 3, (val >> 24) as u8);
    }
}
