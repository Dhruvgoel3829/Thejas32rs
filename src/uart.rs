use crate::gazebo_backend::GazeboUartBackend;

const UART_RHR_THR: u32 = 0x00;
const UART_LSR: u32 = 0x05;

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x1000;

pub struct Uart {
    pub gazebo: GazeboUartBackend,
}

impl Uart {
    pub fn new() -> Self {
        Self {
            gazebo: GazeboUartBackend::new().expect("Failed to create Gazebo UART backend"),
        }
    }

    pub fn read_u8(&mut self, offset: u32) -> u8 {
        match offset {
            UART_RHR_THR => 0,
            UART_LSR => 0x60,
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            UART_RHR_THR => {
                self.gazebo.write(val);
            }
            _ => {}
        }
    }
}