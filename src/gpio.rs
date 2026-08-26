pub const GPIO_BASE: u32 = 0x1001_0000;
pub const GPIO_SIZE: u32 = 0x1000;

// Register Offsets
const GPIO_DATA_IN: u32   = 0x00;
const GPIO_DATA_OUT: u32  = 0x04;
const GPIO_DIR: u32       = 0x08; // 1 = Output, 0 = Input
const GPIO_SET: u32       = 0x0C; // Atomic bit set
const GPIO_CLEAR: u32     = 0x10; // Atomic bit clear

pub struct Gpio {
    pub data_in: u32,
    pub data_out: u32,
    pub direction: u32,
}

impl Gpio {
    pub fn new() -> Self {
        Self {
            data_in: 0,
            data_out: 0,
            direction: 0,
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            GPIO_DATA_IN => (self.data_in & !self.direction) | (self.data_out & self.direction),
            GPIO_DATA_OUT => self.data_out,
            GPIO_DIR => self.direction,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            GPIO_DATA_OUT => self.data_out = val,
            GPIO_DIR => self.direction = val,
            GPIO_SET => self.data_out |= val,
            GPIO_CLEAR => self.data_out &= !val,
            _ => {}
        }
    }
}