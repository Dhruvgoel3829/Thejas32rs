pub const TIMER_BASE: u32 = 0x1002_0000;
pub const TIMER_SIZE: u32 = 0x1000;

// Register Offsets
const TIMER_CTRL: u32    = 0x00; // bit 0: enable, bit 1: irq_en, bit 2: auto_reload
const TIMER_COUNT: u32   = 0x04; // Current counter value
const TIMER_RELOAD: u32  = 0x08; // Compare/Reload value
const TIMER_PRESC: u32   = 0x0C; // Prescaler divider
const TIMER_STATUS: u32  = 0x10; // bit 0: match interrupt flag

pub struct Timer {
    pub ctrl: u32,
    pub count: u32,
    pub reload: u32,
    pub prescaler: u32,
    pub presc_counter: u32,
    pub status: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            ctrl: 0,
            count: 0,
            reload: 0xFFFF_FFFF,
            prescaler: 0,
            presc_counter: 0,
            status: 0,
        }
    }

    /// Advance timer by 1 SoC clock cycle
    pub fn tick(&mut self) -> bool {
        let enabled = (self.ctrl & 0x01) != 0;
        if !enabled {
            return false;
        }

        if self.presc_counter < self.prescaler {
            self.presc_counter += 1;
            return false;
        }
        self.presc_counter = 0;

        self.count = self.count.wrapping_add(1);

        if self.count >= self.reload {
            self.status |= 0x01; // Set match flag
            let auto_reload = (self.ctrl & 0x04) != 0;
            if auto_reload {
                self.count = 0;
            } else {
                self.ctrl &= !0x01; // Disable timer
            }

            let irq_en = (self.ctrl & 0x02) != 0;
            return irq_en;
        }

        false
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            TIMER_CTRL => self.ctrl,
            TIMER_COUNT => self.count,
            TIMER_RELOAD => self.reload,
            TIMER_PRESC => self.prescaler,
            TIMER_STATUS => self.status,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            TIMER_CTRL => self.ctrl = val,
            TIMER_COUNT => self.count = val,
            TIMER_RELOAD => self.reload = val,
            TIMER_PRESC => self.prescaler = val,
            TIMER_STATUS => self.status &= !val, // W1C (Write 1 to clear)
            _ => {}
        }
    }
}