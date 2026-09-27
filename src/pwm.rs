// src/pwm.rs

use crate::ipc::SocketBackend;

pub const PWM_BASE: u32 = 0x1009_0000;
pub const PWM_SIZE: u32 = 0x1000;

// Register Offsets
const PWM_CTRL_OFFSET: u32    = 0x00; // bit 0: enable, bits 1-8: channel enables, bit 16: center-aligned
const PWM_FREQ_OFFSET: u32    = 0x04; // PWM frequency in Hz
const PWM_DUTY0_OFFSET: u32   = 0x10; // Channel 0 duty cycle (0-4095)
const PWM_DUTY1_OFFSET: u32   = 0x14; // Channel 1 duty cycle
const PWM_DUTY2_OFFSET: u32   = 0x18; // Channel 2 duty cycle
const PWM_DUTY3_OFFSET: u32   = 0x1C; // Channel 3 duty cycle
const PWM_DUTY4_OFFSET: u32   = 0x20; // Channel 4 duty cycle
const PWM_DUTY5_OFFSET: u32   = 0x24; // Channel 5 duty cycle
const PWM_DUTY6_OFFSET: u32   = 0x28; // Channel 6 duty cycle
const PWM_DUTY7_OFFSET: u32   = 0x2C; // Channel 7 duty cycle
const PWM_DEAD_OFFSET: u32    = 0x30; // Dead time insertion (ns)
const PWM_PHASE_OFFSET: u32   = 0x34; // Phase shift per channel (0-360 degrees * 100)

const MAX_CHANNELS: usize = 8;
const MAX_DUTY: u16 = 4095; // 12-bit resolution

pub struct Pwm {
    pub ctrl: u32,
    pub freq: u32,
    pub duty: [u16; MAX_CHANNELS],
    pub dead_time: u16,
    pub phase_shift: [u16; MAX_CHANNELS],
    ipc: Option<SocketBackend>,
}

impl Pwm {
    pub fn new(enable_ipc: bool) -> Self {
        Self {
            ctrl: 0,
            freq: 50000, // Default 50 kHz
            duty: [0; MAX_CHANNELS],
            dead_time: 0,
            phase_shift: [0; MAX_CHANNELS],
            ipc: if enable_ipc {
                Some(SocketBackend::new("/tmp/thejas_pwm.sock"))
            } else {
                None
            },
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            PWM_CTRL_OFFSET => self.ctrl,
            PWM_FREQ_OFFSET => self.freq,
            PWM_DUTY0_OFFSET..=PWM_DUTY7_OFFSET => {
                let idx = ((offset - PWM_DUTY0_OFFSET) / 4) as usize;
                if idx < MAX_CHANNELS {
                    self.duty[idx] as u32
                } else {
                    0
                }
            }
            PWM_DEAD_OFFSET => self.dead_time as u32,
            PWM_PHASE_OFFSET..=0x53 => {
                let idx = ((offset - PWM_PHASE_OFFSET) / 4) as usize;
                if idx < MAX_CHANNELS {
                    self.phase_shift[idx] as u32
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            PWM_CTRL_OFFSET => {
                self.ctrl = val;
            }
            PWM_FREQ_OFFSET => {
                self.freq = val.max(1); // Prevent divide by zero
            }
            PWM_DUTY0_OFFSET..=PWM_DUTY7_OFFSET => {
                let idx = ((offset - PWM_DUTY0_OFFSET) / 4) as usize;
                if idx < MAX_CHANNELS {
                    self.duty[idx] = (val & 0xFFFF) as u16;
                    // Clamp to MAX_DUTY
                    if self.duty[idx] > MAX_DUTY {
                        self.duty[idx] = MAX_DUTY;
                    }
                }
            }
            PWM_DEAD_OFFSET => {
                self.dead_time = (val & 0xFFFF) as u16;
            }
            PWM_PHASE_OFFSET..=0x53 => {
                let idx = ((offset - PWM_PHASE_OFFSET) / 4) as usize;
                if idx < MAX_CHANNELS {
                    self.phase_shift[idx] = (val & 0xFFFF) as u16;
                }
            }
            _ => {}
        }

        // If IPC enabled, forward to external device
        if let Some(ref mut ipc) = self.ipc {
            // Send update to external PWM device
            // Protocol: [op, channel, value]
            let op = match offset {
                PWM_CTRL_OFFSET => 0x00,
                PWM_FREQ_OFFSET => 0x01,
                PWM_DUTY0_OFFSET..=PWM_DUTY7_OFFSET => 0x02,
                PWM_DEAD_OFFSET => 0x03,
                PWM_PHASE_OFFSET..=0x53 => 0x04,
                _ => return,
            };
            let channel = match offset {
                PWM_DUTY0_OFFSET..=PWM_DUTY7_OFFSET => ((offset - PWM_DUTY0_OFFSET) / 4) as u8,
                PWM_PHASE_OFFSET..=0x53 => ((offset - PWM_PHASE_OFFSET) / 4) as u8,
                _ => 0,
            };
            let val_bytes = val.to_le_bytes();
            let msg = [op, channel, val_bytes[0], val_bytes[1], val_bytes[2], val_bytes[3]];
            ipc.send(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pwm_new() {
        let pwm = Pwm::new(false);
        assert_eq!(pwm.ctrl, 0);
        assert_eq!(pwm.freq, 50000);
        assert_eq!(pwm.duty, [0; MAX_CHANNELS]);
        assert_eq!(pwm.dead_time, 0);
        assert_eq!(pwm.phase_shift, [0; MAX_CHANNELS]);
        assert!(pwm.ipc.is_none());
    }

    #[test]
    fn test_pwm_ctrl_register() {
        let mut pwm = Pwm::new(false);
        pwm.write_u32(PWM_CTRL_OFFSET, 0x1FF); // Enable + all 8 channels
        assert_eq!(pwm.read_u32(PWM_CTRL_OFFSET), 0x1FF);
        assert_eq!(pwm.ctrl, 0x1FF);
    }

    #[test]
    fn test_pwm_freq_register() {
        let mut pwm = Pwm::new(false);
        pwm.write_u32(PWM_FREQ_OFFSET, 10000); // 10 kHz
        assert_eq!(pwm.read_u32(PWM_FREQ_OFFSET), 10000);
        assert_eq!(pwm.freq, 10000);
    }

    #[test]
    fn test_pwm_freq_minimum() {
        let mut pwm = Pwm::new(false);
        pwm.write_u32(PWM_FREQ_OFFSET, 0); // Should clamp to 1
        assert_eq!(pwm.freq, 1);
    }

    #[test]
    fn test_pwm_duty_registers() {
        let mut pwm = Pwm::new(false);
        
        // Test each channel
        for i in 0..MAX_CHANNELS {
            let offset = PWM_DUTY0_OFFSET + (i * 4) as u32;
            pwm.write_u32(offset, 2048); // 50% duty
            assert_eq!(pwm.read_u32(offset), 2048);
            assert_eq!(pwm.duty[i], 2048);
        }
    }

    #[test]
    fn test_pwm_duty_clamp() {
        let mut pwm = Pwm::new(false);
        pwm.write_u32(PWM_DUTY0_OFFSET, 5000); // Above MAX_DUTY
        assert_eq!(pwm.duty[0], MAX_DUTY);
    }

    #[test]
    fn test_pwm_dead_time() {
        let mut pwm = Pwm::new(false);
        pwm.write_u32(PWM_DEAD_OFFSET, 1000); // 1000 ns dead time
        assert_eq!(pwm.read_u32(PWM_DEAD_OFFSET), 1000);
        assert_eq!(pwm.dead_time, 1000);
    }

    #[test]
    fn test_pwm_phase_shift() {
        let mut pwm = Pwm::new(false);
        
        for i in 0..MAX_CHANNELS {
            let offset = PWM_PHASE_OFFSET + (i * 4) as u32;
            let phase = (i * 4500) as u32; // 45 degree increments * 100
            pwm.write_u32(offset, phase);
            assert_eq!(pwm.read_u32(offset), phase);
            assert_eq!(pwm.phase_shift[i], phase as u16);
        }
    }

    #[test]
    fn test_pwm_unknown_offset() {
        let mut pwm = Pwm::new(false);
        assert_eq!(pwm.read_u32(0x100), 0);
        pwm.write_u32(0x100, 0xFF);
    }
}