pub const FLASH_BASE: u32 = 0x4000_0000;
pub const FLASH_SIZE: usize = 16 * 1024 * 1024; // 16 MB Flash Window

pub struct QspiFlash {
    pub data: Vec<u8>,
}

impl QspiFlash {
    pub fn new() -> Self {
        Self {
            // Flash defaults to 0xFF (erased state)
            data: vec![0xFF; FLASH_SIZE],
        }
    }

    #[inline(always)]
    pub fn read_u8(&self, offset: u32) -> u8 {
        let idx = offset as usize;
        if idx < self.data.len() {
            self.data[idx]
        } else {
            0xFF
        }
    }

    #[inline(always)]
    pub fn read_u16(&self, offset: u32) -> u16 {
        let b0 = self.read_u8(offset) as u16;
        let b1 = self.read_u8(offset + 1) as u16;
        b0 | (b1 << 8)
    }

    #[inline(always)]
    pub fn read_u32(&self, offset: u32) -> u32 {
        let b0 = self.read_u8(offset) as u32;
        let b1 = self.read_u8(offset + 1) as u32;
        let b2 = self.read_u8(offset + 2) as u32;
        let b3 = self.read_u8(offset + 3) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Load raw binary or section payload into Flash
    pub fn load_bytes(&mut self, offset: u32, bytes: &[u8]) {
        let start = offset as usize;
        let end = (start + bytes.len()).min(self.data.len());
        if start < self.data.len() {
            self.data[start..end].copy_from_slice(&bytes[0..(end - start)]);
        }
    }
}