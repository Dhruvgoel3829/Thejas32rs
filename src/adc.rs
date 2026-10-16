// src/adc.rs

use crate::ipc::SocketBackend;

pub const ADC_BASE: u32 = 0x1008_0000;
pub const ADC_SIZE: u32 = 0x1000;

// Register Offsets
const ADC_CTRL_OFFSET: u32    = 0x00; // bit 0: enable, bit 1: continuous, bits 4-7: channel select
const ADC_STATUS_OFFSET: u32  = 0x04; // bit 0: data ready, bit 1: overflow
const ADC_DATA_OFFSET: u32    = 0x08; // 12-bit ADC result
const ADC_CHAN_OFFSET: u32    = 0x0C; // Channel select (0-7)

pub struct Adc {
    pub ctrl: u32,
    pub status: u32,
    pub data: u16,
    pub channel: u8,
    ipc: Option<SocketBackend>,
}

impl Adc {
    pub fn new(enable_ipc: bool) -> Self {
        Self {
            ctrl: 0,
            status: 0,
            data: 0,
            channel: 0,
            ipc: if enable_ipc {
                Some(SocketBackend::new("/tmp/thejas_adc.sock"))
            } else {
                None
            },
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            ADC_CTRL_OFFSET => self.ctrl,
            ADC_STATUS_OFFSET => self.status,
            ADC_DATA_OFFSET => {
                self.status &= !0x01; // Clear data ready
                self.data as u32
            }
            ADC_CHAN_OFFSET => self.channel as u32,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            ADC_CTRL_OFFSET => {
                self.ctrl = val;
                self.channel = ((val >> 4) & 0x0F) as u8; // Channel select in bits 4-7
            }
            ADC_CHAN_OFFSET => {
                self.channel = (val & 0x0F) as u8;
            }
            ADC_STATUS_OFFSET => {
                if (val & 0x01) != 0 {
                    self.status &= !0x01; // W1C data ready
                }
                if (val & 0x02) != 0 {
                    self.status &= !0x02; // W1C overflow
                }
            }
            _ => {}
        }
    }

    /// Simulate ADC conversion (called periodically or on trigger)
    pub fn convert(&mut self) {
        if (self.ctrl & 0x01) == 0 {
            return; // ADC not enabled
        }

        // If IPC enabled, request conversion from external device
        if let Some(ref mut ipc) = self.ipc {
            let req = [0x01, self.channel];
            ipc.send(&req);
            let mut resp = [0u8; 2];
            let n = ipc.recv(&mut resp);
            if n == 2 {
                // Clamp to 12-bit range
                self.data = u16::from_le_bytes(resp).min(4095);
                self.status |= 0x01; // Data ready
                return;
            }
        }

        // Local simulation: return synthetic values based on channel
        // Channel 0: Battery voltage (~3.0V -> 12-bit ADC with 3.3V ref)
        // Channel 1: Current sensor
        // Channel 2: RSSI
        // Channel 3-7: Other analog inputs
        // All values clamped to 12-bit range (0..=4095)
        let raw: u16 = match self.channel {
            0 => 3723, // ~3.0V battery (3723/4095 * 3.3V = 3.0V)
            1 => 2048, // Mid-scale current (zero current)
            2 => 3000, // RSSI
            _ => 0,
        };
        self.data = raw.min(4095);
        self.status |= 0x01; // Data ready

        // If continuous mode disabled, disable after one conversion
        if (self.ctrl & 0x02) == 0 {
            self.ctrl &= !0x01; // Disable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_new() {
        let adc = Adc::new(false);
        assert_eq!(adc.ctrl, 0);
        assert_eq!(adc.status, 0);
        assert_eq!(adc.data, 0);
        assert_eq!(adc.channel, 0);
        assert!(adc.ipc.is_none());
    }

    #[test]
    fn test_adc_ctrl_register() {
        let mut adc = Adc::new(false);
        adc.write_u32(ADC_CTRL_OFFSET, 0x1F); // Enable + continuous + channel 1
        assert_eq!(adc.read_u32(ADC_CTRL_OFFSET), 0x1F);
        assert_eq!(adc.ctrl, 0x1F);
        assert_eq!(adc.channel, 1);
    }

    #[test]
    fn test_adc_channel_register() {
        let mut adc = Adc::new(false);
        adc.write_u32(ADC_CHAN_OFFSET, 5);
        assert_eq!(adc.read_u32(ADC_CHAN_OFFSET), 5);
        assert_eq!(adc.channel, 5);
    }

    #[test]
    fn test_adc_convert() {
        let mut adc = Adc::new(false);
        adc.write_u32(ADC_CTRL_OFFSET, 0x01); // Enable, channel 0
        adc.convert();
        assert_eq!(adc.status & 0x01, 0x01); // Data ready
        assert_eq!(adc.data, 3723); // Battery voltage (~3.0V)
    }

    #[test]
    fn test_adc_channels() {
        let mut adc = Adc::new(false);
        
        adc.write_u32(ADC_CTRL_OFFSET, 0x11); // Enable + channel 1
        adc.convert();
        assert_eq!(adc.data, 2048); // Current sensor
        
        adc.write_u32(ADC_CTRL_OFFSET, 0x21); // Enable + channel 2
        adc.convert();
        assert_eq!(adc.data, 3000); // RSSI
    }

    #[test]
    fn test_adc_disabled() {
        let mut adc = Adc::new(false);
        adc.convert(); // Should not convert when disabled
        assert_eq!(adc.status & 0x01, 0x00); // No data ready
    }

    #[test]
    fn test_adc_status_w1c() {
        let mut adc = Adc::new(false);
        adc.write_u32(ADC_CTRL_OFFSET, 0x01);
        adc.convert();
        assert_eq!(adc.status & 0x01, 0x01);
        
        // Write 1 to clear data ready
        adc.write_u32(ADC_STATUS_OFFSET, 0x01);
        assert_eq!(adc.status & 0x01, 0x00);
    }

    #[test]
    fn test_adc_clamp_12bit() {
        let mut adc = Adc::new(false);
        // All synthetic channels must stay within 12-bit range
        for ch in 0..8u8 {
            adc.write_u32(ADC_CTRL_OFFSET, 0x01 | ((ch as u32) << 4));
            adc.convert();
            assert!(adc.data <= 4095, "channel {} out of 12-bit range", ch);
        }
    }

    #[test]
    fn test_adc_unknown_offset() {
        let mut adc = Adc::new(false);
        assert_eq!(adc.read_u32(0x100), 0);
        adc.write_u32(0x100, 0xFF);
    }
}