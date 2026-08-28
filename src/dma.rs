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