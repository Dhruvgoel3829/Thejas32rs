// src/bus.rs
pub struct Bus {
    pub ram: Vec<u8>, // 256 KB
    // peripherals will be added here later (UART, PLIC, etc.)
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: vec![0; 256 * 1024],
        }
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        // Handle alignment and address routing
        let offset = addr as usize;
        u32::from_le_bytes(self.ram[offset..offset + 4].try_into().unwrap())
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let offset = addr as usize;
        self.ram[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    }
}