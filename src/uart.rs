use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

const UART_RHR_THR: u32 = 0x00; // Rx Buffer (read) / Tx Holding (write)
const UART_LSR: u32     = 0x05; // Line Status Register

pub struct Uart {
    rx_receiver: Receiver<u8>,
    rx_buffer: Option<u8>,
}

impl Uart {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        // Spawn a background thread to read raw host stdin asynchronously
        thread::spawn(move || {
            let mut stdin = io::stdin().lock();
            let mut buf = [0u8; 1];
            while let Ok(n) = stdin.read(&mut buf) {
                if n > 0 {
                    let _ = tx.send(buf[0]);
                }
            }
        });

        Self {
            rx_receiver: rx,
            rx_buffer: None,
        }
    }

    pub fn read_u8(&mut self, offset: u32) -> u8 {
        // Poll channel if buffer is empty
        if self.rx_buffer.is_none() {
            if let Ok(byte) = self.rx_receiver.try_recv() {
                self.rx_buffer = Some(byte);
            }
        }

        match offset {
            UART_RHR_THR => {
                let byte = self.rx_buffer.take().unwrap_or(0);
                byte
            }
            UART_LSR => {
                let mut lsr = 0x60; // TX always empty/ready
                if self.rx_buffer.is_some() {
                    lsr |= 0x01; // Data Ready bit
                }
                lsr
            }
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            UART_RHR_THR => {
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(&[val]);
                let _ = handle.flush();
            }
            _ => {}
        }
    }
}