// src/i2c.rs
// I2C with extended IPC protocol for multi-byte reads (MS5611, HMC5883L)

use crate::ipc::SocketBackend;

pub const I2C_BASE: u32 = 0x1005_0000;
pub const I2C_SIZE: u32 = 0x1000;

// Register Offsets
const I2C_CR: u32   = 0x00;
const I2C_SR: u32   = 0x04;
const I2C_ADDR: u32 = 0x08;
const I2C_DATA: u32 = 0x0C;

// IPC Protocol opcodes
const I2C_IPC_WRITE: u8 = 0x00;  // Write: [0x00, addr, reg?, data...] - reg optional
const I2C_IPC_READ: u8  = 0x01;  // Read:  [0x01, addr, reg?, num_bytes] - reg optional

pub struct I2c {
    pub cr: u32,
    pub sr: u32,
    pub addr: u8,
    pub rx_buf: [u8; 16],  // Buffer for multi-byte reads
    rx_len: u8,
    rx_pos: u8,
    // Virtual 256-byte 24C02 EEPROM at address 0x50
    eeprom_memory: [u8; 256],
    eeprom_ptr: u8,
    eeprom_addr_latch: bool,
    ipc: Option<SocketBackend>,
}

impl I2c {
    pub fn new(enable_ipc: bool) -> Self {
        Self {
            cr: 0,
            sr: 0x04, // Default: ACK received
            addr: 0,
            rx_buf: [0; 16],
            rx_len: 0,
            rx_pos: 0,
            // Virtual 256-byte 24C02 EEPROM at address 0x50
            eeprom_memory: [0xFF; 256],
            eeprom_ptr: 0,
            eeprom_addr_latch: false,
            ipc: if enable_ipc {
                Some(SocketBackend::new("/tmp/thejas_i2c.sock"))
            } else {
                None
            },
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            I2C_CR => self.cr,
            I2C_SR => self.sr,
            I2C_ADDR => self.addr as u32,
            I2C_DATA => {
                if self.rx_pos < self.rx_len {
                    let byte = self.rx_buf[self.rx_pos as usize];
                    self.rx_pos += 1;
                    if self.rx_pos >= self.rx_len {
                        self.sr &= !0x02; // Clear RX Ready when buffer exhausted
                    }
                    return byte as u32;
                }
                // No buffered data - try IPC read for external devices
                if self.addr != 0x50 && self.addr != 0 {
                    // Try multi-byte read (up to 16 bytes)
                    if self.ipc_read_multi(16) {
                        let byte = self.rx_buf[0];
                        self.rx_pos = 1;
                        return byte as u32;
                    }
                    // Fallback: single byte read
                    if self.ipc_read_multi(1) {
                        let byte = self.rx_buf[0];
                        self.rx_pos = 1;
                        return byte as u32;
                    }
                }
                self.sr &= !0x02; // Clear RX Ready
                0
            }
            _ => 0,
        }
    }

    fn ipc_read_multi(&mut self, num_bytes: u8) -> bool {
        if self.addr == 0x50 || self.addr == 0 {
            return false;
        }
        if let Some(ref mut ipc) = self.ipc {
            // Extended read: [0x01, addr, reg?, num_bytes]
            let req = [I2C_IPC_READ, self.addr, self.eeprom_ptr, num_bytes];
            ipc.send(&req);
            let mut resp = [0u8; 16];
            let n = ipc.recv(&mut resp);
            if n == num_bytes as usize {
                self.rx_buf[..num_bytes as usize].copy_from_slice(&resp[..num_bytes as usize]);
                self.rx_len = num_bytes;
                self.rx_pos = 0;
                self.sr |= 0x02; // RX Ready
                return true;
            }
        }
        false
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            I2C_CR => {
                self.cr = val;
                let is_start = (val & 0x02) != 0;
                let is_read = (val & 0x08) != 0;

                if is_start {
                    self.eeprom_addr_latch = false;
                    self.rx_pos = 0;
                    self.rx_len = 0;
                }

                if is_read && self.addr == 0x50 {
                    // Read from virtual EEPROM
                    self.rx_buf[0] = self.eeprom_memory[self.eeprom_ptr as usize];
                    self.rx_len = 1;
                    self.rx_pos = 0;
                    self.sr |= 0x02; // RX Ready
                } else if is_read && self.addr != 0x50 && self.addr != 0 {
                    // Try multi-byte read (up to 16 bytes)
                    if self.ipc_read_multi(16) {
                        return;
                    }
                    // Fallback: single byte
                    if let Some(ref mut ipc) = self.ipc {
                        let req = [I2C_IPC_READ, self.addr, self.eeprom_ptr, 1];
                        ipc.send(&req);
                        let mut resp = [0u8; 1];
                        let n = ipc.recv(&mut resp);
                        if n == 1 {
                            self.rx_buf[0] = resp[0];
                            self.rx_len = 1;
                            self.rx_pos = 0;
                            self.sr |= 0x02; // RX Ready
                        }
                    }
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
                    // Local virtual EEPROM
                    if !self.eeprom_addr_latch {
                        self.eeprom_ptr = byte; // First byte sets internal memory pointer
                        self.eeprom_addr_latch = true;
                    } else {
                        self.eeprom_memory[self.eeprom_ptr as usize] = byte; // Write byte
                        self.eeprom_ptr = self.eeprom_ptr.wrapping_add(1);
                    }
                    self.sr |= 0x04; // ACK
                } else if let Some(ref mut ipc) = self.ipc {
                    // Extended write: [0x00, addr, reg?, data...]
                    let mut req = vec![I2C_IPC_WRITE, self.addr];
                    if !self.eeprom_addr_latch {
                        req.push(self.eeprom_ptr); // register
                        self.eeprom_addr_latch = true;
                    } else {
                        req.push(self.eeprom_ptr); // register (current pointer)
                    }
                    req.push(byte);
                    ipc.send(&req);
                    self.eeprom_ptr = self.eeprom_ptr.wrapping_add(1);
                    self.sr |= 0x04; // ACK
                }
            }
            _ => {}
        }
    }

    // Public method for firmware to trigger multi-byte reads
    pub fn trigger_multi_read(&mut self, num_bytes: u8) -> bool {
        self.ipc_read_multi(num_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2c_new() {
        let i2c = I2c::new(false);
        assert_eq!(i2c.cr, 0);
        assert_eq!(i2c.sr, 0x04); // ACK by default
        assert_eq!(i2c.addr, 0);
        assert_eq!(i2c.rx_buf, [0; 16]);
        assert_eq!(i2c.eeprom_memory, [0xFF; 256]);
        assert_eq!(i2c.eeprom_ptr, 0);
        assert_eq!(i2c.eeprom_addr_latch, false);
        assert!(i2c.ipc.is_none());
    }

    #[test]
    fn test_i2c_control_register() {
        let mut i2c = I2c::new(false);
        
        i2c.write_u32(I2C_CR, 0x1234);
        assert_eq!(i2c.read_u32(I2C_CR), 0x1234);
        assert_eq!(i2c.cr, 0x1234);
    }

    #[test]
    fn test_i2c_address_register() {
        let mut i2c = I2c::new(false);
        
        // Write address 0x50 (virtual EEPROM)
        i2c.write_u32(I2C_ADDR, 0x50);
        assert_eq!(i2c.read_u32(I2C_ADDR), 0x50);
        assert_eq!(i2c.addr, 0x50);
        assert_eq!(i2c.sr & 0x04, 0x04); // ACK set
        
        // Write invalid address
        i2c.write_u32(I2C_ADDR, 0x60);
        assert_eq!(i2c.sr & 0x04, 0x00); // NACK
    }

    #[test]
    fn test_i2c_eeprom_write_read() {
        let mut i2c = I2c::new(false);
        
        // Address the EEPROM
        i2c.write_u32(I2C_ADDR, 0x50);
        
        // Write: first byte sets pointer, second byte is data
        i2c.write_u32(I2C_DATA, 0x10); // Set pointer to 0x10
        assert_eq!(i2c.eeprom_addr_latch, true);
        assert_eq!(i2c.eeprom_ptr, 0x10);
        
        i2c.write_u32(I2C_DATA, 0xAA); // Write 0xAA at 0x10
        assert_eq!(i2c.eeprom_memory[0x10], 0xAA);
        assert_eq!(i2c.eeprom_ptr, 0x11); // Auto-increment
        
        i2c.write_u32(I2C_DATA, 0xBB); // Write 0xBB at 0x11
        assert_eq!(i2c.eeprom_memory[0x11], 0xBB);
        assert_eq!(i2c.eeprom_ptr, 0x12);
        
        // Verify memory contents directly
        assert_eq!(i2c.eeprom_memory[0x10], 0xAA);
        assert_eq!(i2c.eeprom_memory[0x11], 0xBB);
    }

    #[test]
    fn test_i2c_eeprom_addr_latch() {
        let mut i2c = I2c::new(false);
        
        i2c.write_u32(I2C_ADDR, 0x50);
        
        // First write sets pointer
        i2c.write_u32(I2C_DATA, 0x20);
        assert_eq!(i2c.eeprom_ptr, 0x20);
        assert_eq!(i2c.eeprom_addr_latch, true);
        
        // Subsequent writes write data
        i2c.write_u32(I2C_DATA, 0x55);
        assert_eq!(i2c.eeprom_memory[0x20], 0x55);
        assert_eq!(i2c.eeprom_ptr, 0x21);
    }

    #[test]
    fn test_i2c_status_flags() {
        let mut i2c = I2c::new(false);
        
        assert_eq!(i2c.sr & 0x04, 0x04); // Initial ACK
        
        i2c.write_u32(I2C_ADDR, 0x50);
        assert_eq!(i2c.sr & 0x04, 0x04); // ACK for valid device
        
        i2c.write_u32(I2C_CR, 0x08); // Read
        assert_eq!(i2c.sr & 0x02, 0x02); // RX Ready
        
        i2c.read_u32(I2C_DATA); // Read clears RX Ready
        assert_eq!(i2c.sr & 0x02, 0x00);
    }

    #[test]
    fn test_i2c_unknown_offset() {
        let mut i2c = I2c::new(false);
        assert_eq!(i2c.read_u32(0x100), 0);
        i2c.write_u32(0x100, 0xFF);
    }
}