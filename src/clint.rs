pub const CLINT_BASE: u32 = 0x0200_0000;
pub const CLINT_SIZE: u32 = 0x0001_0000; // 64 KB

// Standard RISC-V CLINT Register Offsets
const MTIMECMP_OFFSET: u32 = 0x4000; // 64-bit
const MTIME_OFFSET: u32    = 0xBFF8; // 64-bit

pub struct Clint {
    pub mtime: u64,
    pub mtimecmp: u64,
}

impl Clint {
    pub fn new() -> Self {
        Self {
            mtime: 0,
            mtimecmp: u64::MAX, // Defaults to MAX so interrupt doesn't fire immediately
        }
    }

    /// Advance clock by 1 tick
    pub fn tick(&mut self) {
        self.mtime = self.mtime.wrapping_add(1);
    }

    /// Check if timer interrupt condition is met
    pub fn is_interrupt_pending(&self) -> bool {
        self.mtime >= self.mtimecmp
    }

    pub fn read_u8(&self, offset: u32) -> u8 {
        match offset {
            0x4000..=0x4007 => {
                let shift = (offset - MTIMECMP_OFFSET) * 8;
                ((self.mtimecmp >> shift) & 0xFF) as u8
            }
            0xBFF8..=0xBFFF => {
                let shift = (offset - MTIME_OFFSET) * 8;
                ((self.mtime >> shift) & 0xFF) as u8
            }
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            0x4000..=0x4007 => {
                let shift = (offset - MTIMECMP_OFFSET) * 8;
                let mask = !(0xFFu64 << shift);
                self.mtimecmp = (self.mtimecmp & mask) | ((val as u64) << shift);
            }
            0xBFF8..=0xBFFF => {
                let shift = (offset - MTIME_OFFSET) * 8;
                let mask = !(0xFFu64 << shift);
                self.mtime = (self.mtime & mask) | ((val as u64) << shift);
            }
            _ => {}
        }
    }
}