pub const PLIC_BASE: u32 = 0x0C00_0000;
pub const PLIC_SIZE: u32 = 0x0040_0000; // 4 MB memory map window

pub struct Plic {
    pub priorities: [u32; 32], // Priorities for interrupt sources 0..31
    pub pending: u32,          // Pending bitmask
    pub enabled: u32,          // Enabled bitmask for Hart 0
    pub threshold: u32,        // Priority threshold for Hart 0
    pub claimed: u32,          // Currently claimed interrupt ID
}

impl Plic {
    pub fn new() -> Self {
        Self {
            priorities: [0; 32],
            pending: 0,
            enabled: 0,
            threshold: 0,
            claimed: 0,
        }
    }

    /// Called by peripherals (e.g., UART RX) to raise an external interrupt line
    pub fn set_interrupt_pending(&mut self, source_id: usize) {
        if source_id > 0 && source_id < 32 {
            self.pending |= 1 << source_id;
        }
    }

    /// Evaluates if an external interrupt should be signaled to the CPU
    pub fn is_interrupt_pending(&self) -> bool {
        self.get_highest_priority_id() > 0
    }

    /// Finds the highest-priority enabled pending interrupt that exceeds the threshold
    fn get_highest_priority_id(&self) -> u32 {
        let mut max_priority = self.threshold;
        let mut best_id = 0;

        let active = self.pending & self.enabled;
        for id in 1..32 {
            if (active & (1 << id)) != 0 {
                let prio = self.priorities[id];
                if prio > max_priority {
                    max_priority = prio;
                    best_id = id as u32;
                }
            }
        }

        best_id
    }

    /// Claim the highest-priority interrupt
    pub fn claim(&mut self) -> u32 {
        let id = self.get_highest_priority_id();
        if id > 0 {
            self.pending &= !(1 << id); // Clear pending status upon claim
            self.claimed = id;
        }
        id
    }

    /// Complete the interrupt servicing
    pub fn complete(&mut self, id: u32) {
        if self.claimed == id {
            self.claimed = 0;
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            // Source Priorities: 0x000004 .. 0x00007C
            0x000004..=0x00007C => {
                let idx = (offset / 4) as usize;
                self.priorities[idx]
            }
            // Pending Bits: 0x001000
            0x001000 => self.pending,
            // Interrupt Enables: 0x002000
            0x002000 => self.enabled,
            // Priority Threshold: 0x200000
            0x200000 => self.threshold,
            // Claim / Complete: 0x200004
            0x200004 => self.claim(),
            _ => 0,
        }
    }

    // src/plic.rs (inside write_u32)
    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            // Set Source Priority
            0x000004..=0x00007C => {
                let idx = (offset / 4) as usize;
                self.priorities[idx] = val & 0x7; // Priorities 0..7
            }
            // Software trigger / Pending bits
            0x001000 => {
                self.pending = val;
            }
            // Interrupt Enables
            0x002000 => {
                self.enabled = val;
            }
            // Priority Threshold
            0x200000 => {
                self.threshold = val & 0x7;
            }
            // Claim / Complete (Writing source ID signals completion)
            0x200004 => {
                self.complete(val);
            }
            _ => {}
        }
    }
}
