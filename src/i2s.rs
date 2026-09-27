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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2s_new() {
        let i2s = I2s::new();
        assert_eq!(i2s.cr, 0);
        assert_eq!(i2s.last_left_sample, 0);
        assert_eq!(i2s.last_right_sample, 0);
        assert_eq!(i2s.sample_count, 0);
    }

    #[test]
    fn test_i2s_control_register() {
        let mut i2s = I2s::new();
        
        i2s.write_u32(I2S_CR, 0x03); // Enable + 32-bit
        assert_eq!(i2s.read_u32(I2S_CR), 0x03);
        assert_eq!(i2s.cr, 0x03);
    }

    #[test]
    fn test_i2s_status_register() {
        let i2s = I2s::new();
        // Always ready
        assert_eq!(i2s.read_u32(I2S_SR), 0x01);
    }

    #[test]
    fn test_i2s_audio_samples() {
        let mut i2s = I2s::new();
        
        // Write left sample
        i2s.write_u32(I2S_LEFT_TX, 0x1234_5678);
        assert_eq!(i2s.read_u32(I2S_LEFT_TX), 0x1234_5678);
        assert_eq!(i2s.last_left_sample, 0x1234_5678);
        assert_eq!(i2s.sample_count, 1);
        
        // Write right sample
        i2s.write_u32(I2S_RIGHT_TX, 0xABCD_EF00);
        assert_eq!(i2s.read_u32(I2S_RIGHT_TX), 0xABCD_EF00);
        assert_eq!(i2s.last_right_sample, 0xABCD_EF00);
        assert_eq!(i2s.sample_count, 1); // Only left increments
        
        // Write another left
        i2s.write_u32(I2S_LEFT_TX, 0xFFFF_0000);
        assert_eq!(i2s.sample_count, 2);
    }

    #[test]
    fn test_i2s_unknown_offset() {
        let mut i2s = I2s::new();
        assert_eq!(i2s.read_u32(0x100), 0);
        i2s.write_u32(0x100, 0xFF);
        // Should not panic
    }
}