use std::io::{self, Write};

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x100;

// Standard 16550 UART Register Offsets
const UART_RHR_THR: u32 = 0x00; // Receiver / Transmitter Buffer
const UART_LSR: u32     = 0x05; // Line Status Register

pub struct Uart {
    // 0x20 = Transmitter Holding Register Empty (Ready to send)
    // 0x40 = Transmitter Empty
    lsr: u8,
}

impl Uart {
    pub fn new() -> Self {
        Self {
            lsr: 0x60, // Always ready to transmit
        }
    }

    pub fn read_u8(&self, offset: u32) -> u8 {
        match offset {
            UART_LSR => self.lsr,
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            UART_RHR_THR => {
                // Directly write character to host stdout
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(&[val]);
                let _ = handle.flush();
            }
            _ => {}
        }
    }
}