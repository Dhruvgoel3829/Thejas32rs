// src/bus.rs
use std::fs;
use goblin::elf::Elf;
use crate::uart::Uart;

pub const RAM_BASE: u32 = 0x8000_0000; // Standard C-DAC THEJAS32 SRAM Base
pub const RAM_SIZE: usize = 512 * 1024; // 512 KB SRAM buffer

pub struct Bus {
    pub ram: Vec<u8>,
    pub uart: Uart,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: vec![0; RAM_SIZE],
            uart: Uart::new(),
        }
    }

    pub fn load_elf(&mut self, path: &str) -> u32 {
        let buffer = fs::read(path).expect("Failed to read ELF file");
        let elf = Elf::parse(&buffer).expect("Failed to parse ELF");

        for ph in elf.program_headers.iter() {
            // PT_LOAD = 1
            if ph.p_type == 1 {
                let vaddr = ph.p_vaddr as u32;
                let offset = ph.p_offset as usize;
                let filesz = ph.p_filesz as usize;
                let memsz = ph.p_memsz as usize;

                let segment_data = &buffer[offset..offset + filesz];

                // Write segment data into RAM
                for (i, &byte) in segment_data.iter().enumerate() {
                    self.write_u8(vaddr + i as u32, byte);
                }

                // Zero-fill the remaining BSS section if memsz > filesz
                for i in filesz..memsz {
                    self.write_u8(vaddr + i as u32, 0);
                }
            }
        }

        elf.entry as u32
    }

    pub fn read_u8(&self, addr: u32) -> u8 {
        match addr {
            RAM_BASE..=0x8007_FFFF => {
                let offset = (addr - RAM_BASE) as usize;
                self.ram[offset]
            }
            0x1000_0000..=0x1000_00FF => self.uart.read_u8(addr - 0x1000_0000),
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
            _ => {}
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        let b0 = self.read_u8(addr) as u16;
        let b1 = self.read_u8(addr + 1) as u16;
        b0 | (b1 << 8)
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
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
        self.write_u8(addr, val as u8);
        self.write_u8(addr + 1, (val >> 8) as u8);
        self.write_u8(addr + 2, (val >> 16) as u8);
        self.write_u8(addr + 3, (val >> 24) as u8);
    }
}