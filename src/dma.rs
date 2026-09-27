pub const DMA_BASE: u32 = 0x1006_0000;
pub const DMA_SIZE: u32 = 0x1000;

// Register Offsets
const DMA_SRC_OFFSET: u32    = 0x00;
const DMA_DST_OFFSET: u32    = 0x04;
const DMA_LEN_OFFSET: u32    = 0x08;
const DMA_CTRL_OFFSET: u32   = 0x0C;
const DMA_STATUS_OFFSET: u32 = 0x10;

pub struct Dma {
    pub src: u32,
    pub dst: u32,
    pub len: u32,
    pub ctrl: u32,
    pub status: u32,
    // Internal transfer progress
    pub bytes_remaining: u32,
    pub current_src: u32,
    pub current_dst: u32,
}

impl Dma {
    pub fn new() -> Self {
        Self {
            src: 0,
            dst: 0,
            len: 0,
            ctrl: 0x0C, // Default: SRC_INC (bit 2) and DST_INC (bit 3) enabled
            status: 0,
            bytes_remaining: 0,
            current_src: 0,
            current_dst: 0,
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            DMA_SRC_OFFSET => self.src,
            DMA_DST_OFFSET => self.dst,
            DMA_LEN_OFFSET => self.len,
            DMA_CTRL_OFFSET => self.ctrl,
            DMA_STATUS_OFFSET => self.status,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            DMA_SRC_OFFSET => self.src = val,
            DMA_DST_OFFSET => self.dst = val,
            DMA_LEN_OFFSET => self.len = val,
            DMA_CTRL_OFFSET => {
                self.ctrl = val;
                // If START bit (bit 0) is set, arm transfer
                if (val & 0x01) != 0 && self.len > 0 {
                    self.bytes_remaining = self.len;
                    self.current_src = self.src;
                    self.current_dst = self.dst;
                    self.status |= 0x01;  // Set BUSY (bit 0)
                    self.status &= !0x02; // Clear DONE (bit 1)
                }
            }
            DMA_STATUS_OFFSET => {
                // W1C (Write 1 to clear) for DONE bit (bit 1)
                if (val & 0x02) != 0 {
                    self.status &= !0x02;
                }
            }
            _ => {}
        }
    }

    /// Check if DMA has an active transfer in flight
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        (self.status & 0x01) != 0 && self.bytes_remaining > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_new() {
        let dma = Dma::new();
        assert_eq!(dma.src, 0);
        assert_eq!(dma.dst, 0);
        assert_eq!(dma.len, 0);
        assert_eq!(dma.ctrl, 0x0C); // SRC_INC + DST_INC
        assert_eq!(dma.status, 0);
        assert_eq!(dma.bytes_remaining, 0);
        assert_eq!(dma.current_src, 0);
        assert_eq!(dma.current_dst, 0);
        assert!(!dma.is_active());
    }

    #[test]
    fn test_dma_registers() {
        let mut dma = Dma::new();
        
        dma.write_u32(DMA_SRC_OFFSET, 0x1000);
        dma.write_u32(DMA_DST_OFFSET, 0x2000);
        dma.write_u32(DMA_LEN_OFFSET, 100);
        dma.write_u32(DMA_CTRL_OFFSET, 0x00); // No START yet
        
        assert_eq!(dma.read_u32(DMA_SRC_OFFSET), 0x1000);
        assert_eq!(dma.read_u32(DMA_DST_OFFSET), 0x2000);
        assert_eq!(dma.read_u32(DMA_LEN_OFFSET), 100);
        assert_eq!(dma.read_u32(DMA_CTRL_OFFSET), 0x00);
        assert_eq!(dma.read_u32(DMA_STATUS_OFFSET), 0);
    }

    #[test]
    fn test_dma_start_transfer() {
        let mut dma = Dma::new();
        
        dma.write_u32(DMA_SRC_OFFSET, 0x1000);
        dma.write_u32(DMA_DST_OFFSET, 0x2000);
        dma.write_u32(DMA_LEN_OFFSET, 64);
        
        // Not started yet
        assert!(!dma.is_active());
        
        // Start transfer (set bit 0)
        dma.write_u32(DMA_CTRL_OFFSET, 0x01);
        
        assert!(dma.is_active());
        assert_eq!(dma.bytes_remaining, 64);
        assert_eq!(dma.current_src, 0x1000);
        assert_eq!(dma.current_dst, 0x2000);
        assert_eq!(dma.status & 0x01, 0x01); // BUSY
        assert_eq!(dma.status & 0x02, 0x00); // DONE clear
    }

    #[test]
    fn test_dma_ctrl_bits() {
        let mut dma = Dma::new();
        
        // Default has SRC_INC (bit 2) and DST_INC (bit 3) enabled
        assert_eq!(dma.ctrl & 0x0C, 0x0C);
        
        dma.write_u32(DMA_CTRL_OFFSET, 0x00); // Clear all
        assert_eq!(dma.ctrl, 0x00);
        
        dma.write_u32(DMA_CTRL_OFFSET, 0x0F); // All bits
        assert_eq!(dma.ctrl, 0x0F);
    }

    #[test]
    fn test_dma_status_w1c() {
        let mut dma = Dma::new();
        
        dma.write_u32(DMA_SRC_OFFSET, 0x1000);
        dma.write_u32(DMA_DST_OFFSET, 0x2000);
        dma.write_u32(DMA_LEN_OFFSET, 10);
        dma.write_u32(DMA_CTRL_OFFSET, 0x01); // Start
        
        // Simulate completion by manually setting DONE
        dma.status |= 0x02; // DONE
        assert_eq!(dma.read_u32(DMA_STATUS_OFFSET) & 0x02, 0x02);
        
        // Write 1 to clear DONE (W1C)
        dma.write_u32(DMA_STATUS_OFFSET, 0x02);
        assert_eq!(dma.read_u32(DMA_STATUS_OFFSET) & 0x02, 0x00);
        
        // Write 0 should not clear
        dma.status |= 0x02;
        dma.write_u32(DMA_STATUS_OFFSET, 0x00);
        assert_eq!(dma.read_u32(DMA_STATUS_OFFSET) & 0x02, 0x02);
    }

    #[test]
    fn test_dma_unknown_offset() {
        let mut dma = Dma::new();
        assert_eq!(dma.read_u32(0x100), 0);
        dma.write_u32(0x100, 0xFF);
        // Should not panic
    }
}