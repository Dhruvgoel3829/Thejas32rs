pub const I2S_BASE: u32 = 0x1004_0000;
pub const I2S_SIZE: u32 = 0x1000;

// Register Offsets
const I2S_CR: u32       = 0x00; // Control (bit 0: enable, bit 1: 16/32bit)
const I2S_SR: u32       = 0x04; // Status (bit 0: TX FIFO empty, ready for samples)
const I2S_LEFT_TX: u32  = 0x08; // Left Audio Channel Sample
const I2S_RIGHT_TX: u32 = 0x0C; // Right Audio Channel Sample

pub struct I2s {
    pub cr: u32,
    pub last_left_sample: u32,
    pub last_right_sample: u32,
    pub sample_count: u64,
}

impl I2s {
    pub fn new() -> Self {
        Self {
            cr: 0,
            last_left_sample: 0,
            last_right_sample: 0,
            sample_count: 0,
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            I2S_CR => self.cr,
            I2S_SR => 0x01, // Always ready for audio samples
            I2S_LEFT_TX => self.last_left_sample,
            I2S_RIGHT_TX => self.last_right_sample,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            I2S_CR => self.cr = val,
            I2S_LEFT_TX => {
                self.last_left_sample = val;
                self.sample_count += 1;
            }
            I2S_RIGHT_TX => {
                self.last_right_sample = val;
            }
            _ => {}
        }
    }
}