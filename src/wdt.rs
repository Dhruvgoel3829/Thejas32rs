pub const WDT_BASE: u32 = 0x1007_0000;
pub const WDT_SIZE: u32 = 0x1000;

const WDT_CTRL_OFFSET: u32   = 0x00;
const WDT_COUNT_OFFSET: u32  = 0x04;
const WDT_RELOAD_OFFSET: u32 = 0x08;
const WDT_FEED_OFFSET: u32   = 0x0C;
const WDT_STATUS_OFFSET: u32 = 0x10;

// Magic unlock key commonly used in embedded watchdog feeds
const WDT_FEED_KEY_1: u32 = 0x5555_AAAA;
const WDT_FEED_KEY_2: u32 = 0x76;

#[derive(Default)]
pub struct Wdt {
    pub ctrl: u32,
    pub count: u32,
    pub reload: u32,
    pub status: u32,
}

#[derive(Debug, PartialEq)]
pub enum WdtAction {
    None,
    Interrupt,
    SystemReset,
}

impl Wdt {
    pub fn new() -> Self {
        Self {
            ctrl: 0,
            count: 0xFFFF_FFFF,
            reload: 0xFFFF_FFFF,
            status: 0,
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            WDT_CTRL_OFFSET => self.ctrl,
            WDT_COUNT_OFFSET => self.count,
            WDT_RELOAD_OFFSET => self.reload,
            WDT_FEED_OFFSET => 0,
            WDT_STATUS_OFFSET => self.status,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            WDT_CTRL_OFFSET => {
                self.ctrl = val;
            }
            WDT_COUNT_OFFSET => {
                self.count = val;
            }
            WDT_RELOAD_OFFSET => {
                self.reload = val;
            }
            WDT_FEED_OFFSET => {
                if val == WDT_FEED_KEY_1 || val == WDT_FEED_KEY_2 {
                    self.count = self.reload;
                }
            }
            WDT_STATUS_OFFSET => {
                if (val & 0x01) != 0 {
                    self.status &= !0x01; // W1C timeout flag
                }
            }
            _ => {}
        }
    }

    /// Ticks the watchdog timer. Returns whether a reset or interrupt should occur.
    pub fn tick(&mut self) -> WdtAction {
        // bit 0 = ENABLE
        if (self.ctrl & 0x01) == 0 {
            return WdtAction::None;
        }

        if self.count > 0 {
            self.count -= 1;
            return WdtAction::None;
        }

        // Counter hit zero: trigger timeout
        self.status |= 0x01;
        self.count = self.reload; // Auto-reload

        // bit 1 = RESET_EN
        if (self.ctrl & 0x02) != 0 {
            WdtAction::SystemReset
        } else {
            WdtAction::Interrupt
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wdt_new() {
        let wdt = Wdt::new();
        assert_eq!(wdt.ctrl, 0);
        assert_eq!(wdt.count, 0xFFFF_FFFF);
        assert_eq!(wdt.reload, 0xFFFF_FFFF);
        assert_eq!(wdt.status, 0);
    }

    #[test]
    fn test_wdt_disabled() {
        let mut wdt = Wdt::new();
        // Disabled by default
        assert_eq!(wdt.tick(), WdtAction::None);
        // Count should NOT change when disabled
        assert_eq!(wdt.count, 0xFFFF_FFFF);
        assert_eq!(wdt.status, 0);
    }

    #[test]
    fn test_wdt_enabled_countdown() {
        let mut wdt = Wdt::new();
        wdt.ctrl = 0x01; // Enable
        wdt.reload = 5;
        wdt.count = 5;
        
        for i in (1..=5).rev() {
            assert_eq!(wdt.tick(), WdtAction::None);
            assert_eq!(wdt.count, i - 1);
        }
    }

    #[test]
    fn test_wdt_timeout_interrupt() {
        let mut wdt = Wdt::new();
        wdt.ctrl = 0x01; // Enable, no reset
        wdt.reload = 2;
        wdt.count = 2;
        
        assert_eq!(wdt.tick(), WdtAction::None); // count=1
        assert_eq!(wdt.tick(), WdtAction::None); // count=0
        assert_eq!(wdt.tick(), WdtAction::Interrupt); // timeout!
        
        assert_eq!(wdt.status & 0x01, 0x01); // Timeout flag set
        assert_eq!(wdt.count, 2); // Auto-reloaded
    }

    #[test]
    fn test_wdt_timeout_reset() {
        let mut wdt = Wdt::new();
        wdt.ctrl = 0x03; // Enable + Reset enable
        wdt.reload = 1;
        wdt.count = 1;
        
        assert_eq!(wdt.tick(), WdtAction::None); // count=0
        assert_eq!(wdt.tick(), WdtAction::SystemReset); // timeout -> reset
        
        assert_eq!(wdt.status & 0x01, 0x01);
        assert_eq!(wdt.count, 1); // Auto-reloaded
    }

    #[test]
    fn test_wdt_feed() {
        let mut wdt = Wdt::new();
        wdt.ctrl = 0x01;
        wdt.reload = 100;
        wdt.count = 50;
        
        // Feed with key 1
        wdt.write_u32(WDT_FEED_OFFSET, WDT_FEED_KEY_1);
        assert_eq!(wdt.count, 100); // Reloaded
        
        wdt.count = 25;
        wdt.write_u32(WDT_FEED_OFFSET, WDT_FEED_KEY_2);
        assert_eq!(wdt.count, 100); // Reloaded
        
        // Wrong key should not feed
        wdt.count = 10;
        wdt.write_u32(WDT_FEED_OFFSET, 0x1234);
        assert_eq!(wdt.count, 10); // Unchanged
    }

    #[test]
    fn test_wdt_status_w1c() {
        let mut wdt = Wdt::new();
        wdt.status |= 0x01; // Set timeout flag
        assert_eq!(wdt.read_u32(WDT_STATUS_OFFSET) & 0x01, 0x01);
        
        // Write 1 to clear
        wdt.write_u32(WDT_STATUS_OFFSET, 0x01);
        assert_eq!(wdt.read_u32(WDT_STATUS_OFFSET) & 0x01, 0x00);
    }

    #[test]
    fn test_wdt_registers() {
        let mut wdt = Wdt::new();
        
        wdt.write_u32(WDT_CTRL_OFFSET, 0x1234);
        assert_eq!(wdt.read_u32(WDT_CTRL_OFFSET), 0x1234);
        
        wdt.write_u32(WDT_COUNT_OFFSET, 0x5678);
        assert_eq!(wdt.read_u32(WDT_COUNT_OFFSET), 0x5678);
        
        wdt.write_u32(WDT_RELOAD_OFFSET, 0x9ABC);
        assert_eq!(wdt.read_u32(WDT_RELOAD_OFFSET), 0x9ABC);
    }

    #[test]
    fn test_wdt_unknown_offset() {
        let mut wdt = Wdt::new();
        assert_eq!(wdt.read_u32(0x100), 0);
        wdt.write_u32(0x100, 0xFF);
        // Should not panic
    }
}