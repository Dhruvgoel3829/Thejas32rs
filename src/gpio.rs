use crate::gazebo_backend::GazeboMotorBackend;

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
    pub gazebo: GazeboMotorBackend,
}

impl Gpio {
    pub fn new() -> Self {
        Self {
            dir: 0,
            data: 0,
            int_en: 0,
            gazebo: GazeboMotorBackend::new().expect("Failed to create Gazebo motor backend"),
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
                self.gazebo.send_motors(val);
            }
            GPIO_SET_OFFSET => {
                self.data |= val;
                self.gazebo.send_motors(self.data);
            }
            GPIO_CLEAR_OFFSET => {
                self.data &= !val;
                self.gazebo.send_motors(self.data);
            }
            GPIO_INT_EN_OFFSET => self.int_en = val,
            _ => {}
        }
    }
}