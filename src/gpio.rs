use crate::ipc::SocketBackend;

pub const GPIO_BASE: u32 = 0x1001_0000;
pub const GPIO_SIZE: u32 = 0x1000;

const GPIO_DIR_OFFSET: u32    = 0x00;
const GPIO_DATA_OFFSET: u32   = 0x04;
const GPIO_SET_OFFSET: u32    = 0x08;
const GPIO_CLEAR_OFFSET: u32  = 0x0C;
const GPIO_INT_EN_OFFSET: u32 = 0x10;

pub struct Gpio {
    pub dir: u32,
    pub data: u32,
    pub int_en: u32,
    ipc: Option<SocketBackend>,
}

impl Gpio {
    pub fn new(enable_ipc: bool) -> Self {
        Self {
            dir: 0,
            data: 0,
            int_en: 0,
            ipc: if enable_ipc {
                Some(SocketBackend::new("/tmp/thejas_gpio.sock"))
            } else {
                None
            },
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            GPIO_DIR_OFFSET => self.dir,
            GPIO_DATA_OFFSET => self.data,
            GPIO_INT_EN_OFFSET => self.int_en,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            GPIO_DIR_OFFSET => self.dir = val,
            GPIO_DATA_OFFSET => {
                self.data = val;
                if let Some(ref mut ipc) = self.ipc {
                    ipc.send(&val.to_le_bytes());
                }
            }
            GPIO_SET_OFFSET => {
                self.data |= val;
                if let Some(ref mut ipc) = self.ipc {
                    ipc.send(&self.data.to_le_bytes());
                }
            }
            GPIO_CLEAR_OFFSET => {
                self.data &= !val;
                if let Some(ref mut ipc) = self.ipc {
                    ipc.send(&self.data.to_le_bytes());
                }
            }
            GPIO_INT_EN_OFFSET => self.int_en = val,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpio_new() {
        let gpio = Gpio::new(false);
        assert_eq!(gpio.dir, 0);
        assert_eq!(gpio.data, 0);
        assert_eq!(gpio.int_en, 0);
        assert!(gpio.ipc.is_none());
    }

    #[test]
    fn test_gpio_dir_register() {
        let mut gpio = Gpio::new(false);
        
        gpio.write_u32(GPIO_DIR_OFFSET, 0xFF);
        assert_eq!(gpio.read_u32(GPIO_DIR_OFFSET), 0xFF);
        assert_eq!(gpio.dir, 0xFF);
    }

    #[test]
    fn test_gpio_data_register() {
        let mut gpio = Gpio::new(false);
        
        gpio.write_u32(GPIO_DATA_OFFSET, 0xAA);
        assert_eq!(gpio.read_u32(GPIO_DATA_OFFSET), 0xAA);
        assert_eq!(gpio.data, 0xAA);
    }

    #[test]
    fn test_gpio_set_clear() {
        let mut gpio = Gpio::new(false);
        
        gpio.write_u32(GPIO_DATA_OFFSET, 0x00);
        assert_eq!(gpio.data, 0x00);
        
        // SET bits
        gpio.write_u32(GPIO_SET_OFFSET, 0x0F);
        assert_eq!(gpio.data, 0x0F);
        
        gpio.write_u32(GPIO_SET_OFFSET, 0xF0);
        assert_eq!(gpio.data, 0xFF);
        
        // CLEAR bits
        gpio.write_u32(GPIO_CLEAR_OFFSET, 0x0F);
        assert_eq!(gpio.data, 0xF0);
        
        gpio.write_u32(GPIO_CLEAR_OFFSET, 0xF0);
        assert_eq!(gpio.data, 0x00);
    }

    #[test]
    fn test_gpio_int_en() {
        let mut gpio = Gpio::new(false);
        
        gpio.write_u32(GPIO_INT_EN_OFFSET, 0x55);
        assert_eq!(gpio.read_u32(GPIO_INT_EN_OFFSET), 0x55);
        assert_eq!(gpio.int_en, 0x55);
    }

    #[test]
    fn test_gpio_unknown_offset() {
        let mut gpio = Gpio::new(false);
        assert_eq!(gpio.read_u32(0x100), 0);
        gpio.write_u32(0x100, 0xFF);
        // Should not panic
    }
}