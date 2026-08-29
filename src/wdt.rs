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