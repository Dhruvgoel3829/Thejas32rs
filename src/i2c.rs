// src/i2c.rs

pub const I2C_BASE: u32 = 0x1005_0000;
pub const I2C_SIZE: u32 = 0x1000;

// Register Offsets
const I2C_CR: u32   = 0x00;
const I2C_SR: u32   = 0x04;
const I2C_ADDR: u32 = 0x08;
const I2C_DATA: u32 = 0x0C;

pub struct I2c {
    pub cr: u32,
    pub sr: u32,
    pub addr: u8,
    pub rx_buf: u8,
    // Virtual 256-byte 24C02 EEPROM at address 0x50
    eeprom_memory: [u8; 256],
    eeprom_ptr: u8,
    eeprom_addr_latch: bool,
}

impl I2c {
    pub fn new() -> Self {
        Self {
            cr: 0,
            sr: 0x04, // Default: ACK received
            addr: 0,
            rx_buf: 0,
            eeprom_memory: [0xFF; 256],
            eeprom_ptr: 0,
            eeprom_addr_latch: false,
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            I2C_CR => self.cr,
            I2C_SR => self.sr,
            I2C_ADDR => self.addr as u32,
            I2C_DATA => {
                self.sr &= !0x02; // Clear RX Ready
                self.rx_buf as u32
            }
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            I2C_CR => {
                self.cr = val;
                let is_start = (val & 0x02) != 0;
                let is_read = (val & 0x08) != 0;

                if is_start {
                    self.eeprom_addr_latch = false;
                }

                if is_read && self.addr == 0x50 {
                    // Read from virtual EEPROM
                    self.rx_buf = self.eeprom_memory[self.eeprom_ptr as usize];
                    self.eeprom_ptr = self.eeprom_ptr.wrapping_add(1);
                    self.sr |= 0x02; // RX Ready
                }
            }
            I2C_ADDR => {
                self.addr = (val & 0x7F) as u8;
                // Assert ACK flag if addressing valid mock device (0x50)
                if self.addr == 0x50 {
                    self.sr |= 0x04; // ACK
                } else {
                    self.sr &= !0x04; // NACK
                }
            }
            I2C_DATA => {
                let byte = (val & 0xFF) as u8;
                if self.addr == 0x50 {
                    if !self.eeprom_addr_latch {
                        self.eeprom_ptr = byte; // First byte sets internal memory pointer
                        self.eeprom_addr_latch = true;
                    } else {
                        self.eeprom_memory[self.eeprom_ptr as usize] = byte; // Write byte
                        self.eeprom_ptr = self.eeprom_ptr.wrapping_add(1);
                    }
                    self.sr |= 0x04; // ACK
                }
            }
            _ => {}
        }
    }
}