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
    pub ipc: SocketBackend,
}

impl Gpio {
    pub fn new() -> Self {
        Self {
            dir: 0,
            data: 0,
            int_en: 0,
            ipc: SocketBackend::new("/tmp/thejas_gpio.sock"),
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        // Poll for external inputs (e.g. sensor DRDY interrupt pins)
        let mut in_buf = [0u8; 4];
        if self.ipc.recv(&mut in_buf) == 4 {
            let ext_in = u32::from_le_bytes(in_buf);
            let input_mask = !self.dir;
            self.data = (self.data & self.dir) | (ext_in & input_mask);
        }

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
                // Broadcast pin changes (e.g. motor PWM / DShot signals) to Gazebo bridge
                self.ipc.send(&self.data.to_le_bytes());
            }
            GPIO_SET_OFFSET => {
                self.data |= val;
                self.ipc.send(&self.data.to_le_bytes());
            }
            GPIO_CLEAR_OFFSET => {
                self.data &= !val;
                self.ipc.send(&self.data.to_le_bytes());
            }
            GPIO_INT_EN_OFFSET => self.int_en = val,
            _ => {}
        }
    }
}