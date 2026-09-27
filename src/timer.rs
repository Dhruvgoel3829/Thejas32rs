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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_new() {
        let timer = Timer::new();
        assert_eq!(timer.ctrl, 0);
        assert_eq!(timer.count, 0);
        assert_eq!(timer.reload, 0xFFFF_FFFF);
        assert_eq!(timer.prescaler, 0);
        assert_eq!(timer.presc_counter, 0);
        assert_eq!(timer.status, 0);
    }

    #[test]
    fn test_timer_disabled() {
        let mut timer = Timer::new();
        // Timer disabled by default
        assert!(!timer.tick());
        assert_eq!(timer.count, 0);
        assert_eq!(timer.status, 0);
    }

    #[test]
    fn test_timer_basic_counting() {
        let mut timer = Timer::new();
        timer.ctrl = 0x01; // Enable
        timer.reload = 10;
        
        // Ticks 1-9: no match (count < reload)
        for i in 1..10 {
            assert!(!timer.tick());
            assert_eq!(timer.count, i);
            assert_eq!(timer.status, 0);
        }
        
        // Tick 10: count=10 >= reload=10 -> match!
        let irq = timer.tick();
        assert!(!irq); // IRQ not enabled
        assert_eq!(timer.count, 10);
        assert_eq!(timer.status, 0x01);
    }

    #[test]
    fn test_timer_match_irq() {
        let mut timer = Timer::new();
        timer.ctrl = 0x03; // Enable + IRQ enable
        timer.reload = 5;
        
        // Ticks 1-4: no match
        for _ in 1..5 {
            assert!(!timer.tick());
        }
        
        // Tick 5: match -> IRQ
        let irq = timer.tick();
        assert!(irq);
        assert_eq!(timer.status, 0x01);
    }

    #[test]
    fn test_timer_auto_reload() {
        let mut timer = Timer::new();
        timer.ctrl = 0x05; // Enable + Auto-reload
        timer.reload = 3;
        
        // First cycle: ticks 1-2 no match, tick 3 match
        assert!(!timer.tick()); // count=1
        assert!(!timer.tick()); // count=2
        let irq = timer.tick(); // count=3 >= 3 -> match
        assert!(!irq); // IRQ not enabled
        assert_eq!(timer.count, 0); // Auto-reload to 0
        assert_eq!(timer.status, 0x01);
        
        // Second cycle
        assert!(!timer.tick()); // count=1
        assert!(!timer.tick()); // count=2
        let irq = timer.tick(); // count=3 >= 3 -> match
        assert!(!irq);
        assert_eq!(timer.count, 0);
        assert_eq!(timer.status, 0x01);
    }

    #[test]
    fn test_timer_no_auto_reload() {
        let mut timer = Timer::new();
        timer.ctrl = 0x01; // Enable only (no auto-reload)
        timer.reload = 2;
        
        assert!(!timer.tick()); // count=1
        let irq = timer.tick(); // count=2 >= 2 -> match, disable
        assert!(!irq);
        assert_eq!(timer.ctrl & 0x01, 0); // Disabled
        assert_eq!(timer.count, 2);
        assert_eq!(timer.status, 0x01);
        
        // Timer should be disabled now
        assert!(!timer.tick());
        assert!(!timer.tick());
        assert_eq!(timer.count, 2);
    }

    #[test]
    fn test_timer_prescaler() {
        let mut timer = Timer::new();
        timer.ctrl = 0x01;
        timer.reload = 2;
        timer.prescaler = 2; // Divide by 3 (0, 1, 2)
        
        // prescaler=2 means: tick, tick, tick -> count++
        // So 3 ticks per count increment
        // Tick 1: presc_counter=1, count=0
        // Tick 2: presc_counter=2, count=0
        // Tick 3: presc_counter=0, count=1
        for _ in 0..3 {
            assert!(!timer.tick());
        }
        assert_eq!(timer.count, 1);
        
        // Next 3 ticks: count=2 -> match!
        for _ in 0..3 {
            assert!(!timer.tick());
        }
        // Tick 6: count increments to 2, then 2 >= 2 -> match!
        assert_eq!(timer.count, 2);
        assert_eq!(timer.status, 0x01);
        assert_eq!(timer.ctrl & 0x01, 0); // Disabled
    }

    #[test]
    fn test_timer_read_write() {
        let mut timer = Timer::new();
        
        timer.write_u32(TIMER_CTRL, 0x1234_5678);
        assert_eq!(timer.read_u32(TIMER_CTRL), 0x1234_5678);
        
        timer.write_u32(TIMER_COUNT, 0xABCD_EF00);
        assert_eq!(timer.read_u32(TIMER_COUNT), 0xABCD_EF00);
        
        timer.write_u32(TIMER_RELOAD, 0x1000);
        assert_eq!(timer.read_u32(TIMER_RELOAD), 0x1000);
        
        timer.write_u32(TIMER_PRESC, 0x10);
        assert_eq!(timer.read_u32(TIMER_PRESC), 0x10);
    }

    #[test]
    fn test_timer_status_w1c() {
        let mut timer = Timer::new();
        timer.ctrl = 0x05; // Enable + Auto-reload
        timer.reload = 2;
        timer.tick(); // count=1
        timer.tick(); // count=2 -> match, status=1
        
        assert_eq!(timer.read_u32(TIMER_STATUS), 0x01);
        
        // Write 1 to clear (W1C)
        timer.write_u32(TIMER_STATUS, 0x01);
        assert_eq!(timer.read_u32(TIMER_STATUS), 0x00);
        
        // Write 0 should not clear
        timer.tick(); // count=1 (auto-reloaded to 0, then incremented)
        timer.tick(); // count=2 -> match again, status=1
        assert_eq!(timer.read_u32(TIMER_STATUS), 0x01);
        timer.write_u32(TIMER_STATUS, 0x00);
        assert_eq!(timer.read_u32(TIMER_STATUS), 0x01);
    }

    #[test]
    fn test_timer_unknown_offset() {
        let mut timer = Timer::new();
        assert_eq!(timer.read_u32(0x100), 0);
        timer.write_u32(0x100, 0xFF);
        // Should not panic
    }
}