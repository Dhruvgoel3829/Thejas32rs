// src/clint.rs

pub const CLINT_BASE: u32 = 0x0200_0000;
pub const CLINT_SIZE: u32 = 0x10000;

// Standard RISC-V CLINT Register Offsets
const MSIP_OFFSET: u32 = 0x0000; // Software interrupt register (Hart 0)
const MTIMECMP_OFFSET: u32 = 0x4000; // Timer compare low 32 bits
const MTIMECMPH_OFFSET: u32 = 0x4004; // Timer compare high 32 bits
const MTIME_OFFSET: u32 = 0xBFF8; // Timer counter low 32 bits
const MTIMEH_OFFSET: u32 = 0xBFFC; // Timer counter high 32 bits

#[derive(Default)]
pub struct Clint {
    pub msip: u32,
    pub mtimecmp: u64,
    pub mtime: u64,
}

impl Clint {
    pub fn new() -> Self {
        Self {
            msip: 0,
            mtimecmp: 0xFFFF_FFFF_FFFF_FFFF,
            mtime: 0,
        }
    }

    /// Increments the hardware timer counter on every cycle/tick
    pub fn tick(&mut self) {
        self.mtime = self.mtime.wrapping_add(1);
    }

    /// Check if machine timer interrupt is pending (mtime >= mtimecmp)
    pub fn is_timer_interrupt_pending(&self) -> bool {
        self.mtime >= self.mtimecmp
    }

    /// Check if machine software interrupt is pending (msip bit 0 != 0)
    pub fn is_software_interrupt_pending(&self) -> bool {
        (self.msip & 0x01) != 0
    }

    pub fn is_interrupt_pending(&self) -> bool {
        self.is_timer_interrupt_pending()
    }

    // -------------------------------------------------------------
    // 32-Bit Word Access
    // -------------------------------------------------------------

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            MSIP_OFFSET => self.msip,
            MTIMECMP_OFFSET => (self.mtimecmp & 0xFFFF_FFFF) as u32,
            MTIMECMPH_OFFSET => (self.mtimecmp >> 32) as u32,
            MTIME_OFFSET => (self.mtime & 0xFFFF_FFFF) as u32,
            MTIMEH_OFFSET => (self.mtime >> 32) as u32,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            MSIP_OFFSET => {
                self.msip = val & 0x01;
            }
            MTIMECMP_OFFSET => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF_0000_0000) | (val as u64);
            }
            MTIMECMPH_OFFSET => {
                self.mtimecmp = (self.mtimecmp & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            }
            MTIME_OFFSET => {
                self.mtime = (self.mtime & 0xFFFF_FFFF_0000_0000) | (val as u64);
            }
            MTIMEH_OFFSET => {
                self.mtime = (self.mtime & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            }
            _ => {}
        }
    }

    // -------------------------------------------------------------
    // 8-Bit Byte Access (Delegates through 32-bit registers)
    // -------------------------------------------------------------

    pub fn read_u8(&self, offset: u32) -> u8 {
        let word_offset = offset & !0x3;
        let shift = (offset & 0x3) * 8;
        ((self.read_u32(word_offset) >> shift) & 0xFF) as u8
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        let word_offset = offset & !0x3;
        let shift = (offset & 0x3) * 8;
        let mut current = self.read_u32(word_offset);
        current = (current & !(0xFF << shift)) | ((val as u32) << shift);
        self.write_u32(word_offset, current);
    }
}
