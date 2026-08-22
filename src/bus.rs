use crate::uart::{Uart, UART_BASE, UART_SIZE};

pub const RAM_BASE: u32 = 0x0000_0000;
pub const RAM_SIZE: usize = 256 * 1024; // 256 KB

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

    pub fn read_u8(&self, addr: u32) -> u8 {
        match addr {
            RAM_BASE..=0x0003_FFFF => self.ram[addr as usize],
            UART_BASE..=0x1000_00FF => self.uart.read_u8(addr - UART_BASE),
            _ => 0,
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

    pub fn write_u8(&mut self, addr: u32, val: u8) {
        match addr {
            RAM_BASE..=0x0003_FFFF => self.ram[addr as usize] = val,
            UART_BASE..=0x1000_00FF => self.uart.write_u8(addr - UART_BASE, val),
            _ => {}
        }
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