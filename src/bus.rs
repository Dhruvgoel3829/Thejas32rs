
pub struct Bus {
    pub ram: Vec<u8>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: vec![0; 256 * 1024], // 256 KB
        }
    }

    pub fn read_u8(&self, addr: u32) -> u8 {
        self.ram[addr as usize]
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        let offset = addr as usize;
        u16::from_le_bytes(self.ram[offset..offset + 2].try_into().unwrap())
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        let offset = addr as usize;
        u32::from_le_bytes(self.ram[offset..offset + 4].try_into().unwrap())
    }

    pub fn write_u8(&mut self, addr: u32, val: u8) {
        self.ram[addr as usize] = val;
    }

    pub fn write_u16(&mut self, addr: u32, val: u16) {
        let offset = addr as usize;
        self.ram[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let offset = addr as usize;
        self.ram[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    }
}