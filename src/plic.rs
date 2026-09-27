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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plic_new() {
        let plic = Plic::new();
        assert_eq!(plic.priorities, [0; 32]);
        assert_eq!(plic.pending, 0);
        assert_eq!(plic.enabled, 0);
        assert_eq!(plic.threshold, 0);
        assert_eq!(plic.claimed, 0);
    }

    #[test]
    fn test_plic_set_interrupt_pending() {
        let mut plic = Plic::new();
        
        // Invalid source IDs (0 and >= 32) should be ignored
        plic.set_interrupt_pending(0);
        assert_eq!(plic.pending, 0);
        
        plic.set_interrupt_pending(32);
        assert_eq!(plic.pending, 0);
        
        // Valid source ID
        plic.set_interrupt_pending(1);
        assert_eq!(plic.pending, 0b10);
        
        plic.set_interrupt_pending(5);
        assert_eq!(plic.pending, 0b10_0010);
        
        plic.set_interrupt_pending(31);
        // bit 1 (0x2) + bit 5 (0x20) + bit 31 (0x80000000) = 0x80000022
        assert_eq!(plic.pending, 0x8000_0022);
    }

    #[test]
    fn test_plic_priority_and_enable() {
        let mut plic = Plic::new();
        
        // Set priorities
        plic.write_u32(0x04, 5); // Source 1 priority = 5
        plic.write_u32(0x08, 3); // Source 2 priority = 3
        plic.write_u32(0x0C, 7); // Source 3 priority = 7
        
        assert_eq!(plic.priorities[1], 5);
        assert_eq!(plic.priorities[2], 3);
        assert_eq!(plic.priorities[3], 7);
        
        // Only lower 3 bits used for priority
        plic.write_u32(0x10, 0xFF);
        assert_eq!(plic.priorities[4], 0x7);
    }

    #[test]
    fn test_plic_pending_register() {
        let mut plic = Plic::new();
        
        plic.write_u32(0x1000, 0x0F); // Set pending bits 0-3
        assert_eq!(plic.read_u32(0x1000), 0x0F);
        assert_eq!(plic.pending, 0x0F);
    }

    #[test]
    fn test_plic_enable_register() {
        let mut plic = Plic::new();
        
        plic.write_u32(0x2000, 0xF0); // Enable bits 4-7
        assert_eq!(plic.read_u32(0x2000), 0xF0);
        assert_eq!(plic.enabled, 0xF0);
    }

    #[test]
    fn test_plic_threshold() {
        let mut plic = Plic::new();
        
        plic.write_u32(0x200000, 4);
        assert_eq!(plic.read_u32(0x200000), 4);
        assert_eq!(plic.threshold, 4);
        
        // Only lower 3 bits
        plic.write_u32(0x200000, 0xFF);
        assert_eq!(plic.threshold, 0x7);
    }

    #[test]
    fn test_plic_is_interrupt_pending() {
        let mut plic = Plic::new();
        
        // No enabled interrupts
        assert!(!plic.is_interrupt_pending());
        
        // Enable source 1, priority 5
        plic.write_u32(0x04, 5);
        plic.write_u32(0x2000, 0x2); // Enable source 1
        plic.set_interrupt_pending(1);
        
        // Priority (5) > threshold (0) -> interrupt pending
        assert!(plic.is_interrupt_pending());
        
        // Raise threshold above priority
        plic.write_u32(0x200000, 6);
        assert!(!plic.is_interrupt_pending());
    }

    #[test]
    fn test_plic_claim() {
        let mut plic = Plic::new();
        
        // Set up multiple interrupts
        plic.write_u32(0x04, 3); // Source 1, prio 3
        plic.write_u32(0x08, 5); // Source 2, prio 5
        plic.write_u32(0x0C, 4); // Source 3, prio 4
        plic.write_u32(0x2000, 0xE); // Enable 1, 2, 3
        plic.set_interrupt_pending(1);
        plic.set_interrupt_pending(2);
        plic.set_interrupt_pending(3);
        
        // Claim should return highest priority (source 2, prio 5)
        let claimed = plic.claim();
        assert_eq!(claimed, 2);
        assert_eq!(plic.claimed, 2);
        assert_eq!(plic.pending, 0xA); // Source 2 cleared, 1 and 3 remain
        
        // Next claim should return source 3 (prio 4)
        let claimed = plic.claim();
        assert_eq!(claimed, 3);
        assert_eq!(plic.pending, 0x2); // Only source 1 remains
        
        // Next claim should return source 1 (prio 3)
        let claimed = plic.claim();
        assert_eq!(claimed, 1);
        assert_eq!(plic.pending, 0);
        
        // No more interrupts
        let claimed = plic.claim();
        assert_eq!(claimed, 0);
    }

    #[test]
    fn test_plic_complete() {
        let mut plic = Plic::new();
        
        plic.write_u32(0x04, 5);
        plic.write_u32(0x2000, 0x2);
        plic.set_interrupt_pending(1);
        
        let claimed = plic.claim();
        assert_eq!(claimed, 1);
        assert_eq!(plic.claimed, 1);
        
        // Complete with wrong ID should not clear claimed
        plic.complete(2);
        assert_eq!(plic.claimed, 1);
        
        // Complete with correct ID
        plic.complete(1);
        assert_eq!(plic.claimed, 0);
    }

    #[test]
    fn test_plic_claim_complete_register() {
        let mut plic = Plic::new();
        
        plic.write_u32(0x04, 5);
        plic.write_u32(0x2000, 0x2);
        plic.set_interrupt_pending(1);
        
        // Read claim/complete register triggers claim
        let claimed = plic.read_u32(0x200004);
        assert_eq!(claimed, 1);
        
        // Write to claim/complete register triggers complete
        plic.write_u32(0x200004, 1);
        assert_eq!(plic.claimed, 0);
    }

    #[test]
    fn test_plic_unknown_offset() {
        let mut plic = Plic::new();
        assert_eq!(plic.read_u32(0x100), 0);
        plic.write_u32(0x100, 0xFF);
        // Should not panic
    }
}
