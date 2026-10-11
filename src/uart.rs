use crate::ipc::SocketBackend;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

pub const UART0_BASE: u32 = 0x1000_0000;
pub const UART1_BASE: u32 = 0x1000_1000;
pub const UART2_BASE: u32 = 0x1000_2000;
pub const UART_SIZE: u32 = 0x1000;

const UART_RHR_THR: u32 = 0x00;
const UART_LSR: u32 = 0x05;

pub struct Uart {
    rx_receiver: Receiver<u8>,
    rx_buffer: Option<u8>,
    ipc: Option<SocketBackend>,
}

impl Uart {
    pub fn new(socket_path: Option<&str>) -> Self {
        let (tx, rx) = mpsc::channel();

        // Only spawn the terminal input thread for UART0 (console)
        if socket_path.is_some() {
            thread::spawn(move || {
                loop {
                    if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                        if let Ok(Event::Key(key_event)) = event::read() {
                            if key_event.kind == crossterm::event::KeyEventKind::Press {
                                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                                    && key_event.code == KeyCode::Char('c')
                                {
                                    let _ = crossterm::terminal::disable_raw_mode();
                                    std::process::exit(0);
                                }

                                let byte_to_send = match key_event.code {
                                    KeyCode::Char(c) => Some(c as u8),
                                    KeyCode::Enter => Some(b'\r'),
                                    KeyCode::Backspace => Some(0x7F),
                                    KeyCode::Tab => Some(b'\t'),
                                    KeyCode::Esc => Some(0x1B),
                                    _ => None,
                                };

                                if let Some(byte) = byte_to_send {
                                    let _ = tx.send(byte);
                                }
                            }
                        }
                    }
                }
            });
        }

        Self {
            rx_receiver: rx,
            rx_buffer: None,
            ipc: socket_path.map(SocketBackend::new),
        }
    }

    pub fn read_u8(&mut self, offset: u32) -> u8 {
        if self.rx_buffer.is_none() {
            if let Ok(byte) = self.rx_receiver.try_recv() {
                self.rx_buffer = Some(byte);
            }
            // If IPC enabled, also try to read from external UART device
            if let Some(ref mut ipc) = self.ipc {
                let mut buf = [0u8; 1];
                let n = ipc.recv(&mut buf);
                if n == 1 && self.rx_buffer.is_none() {
                    self.rx_buffer = Some(buf[0]);
                }
            }
        }

        match offset {
            UART_RHR_THR => self.rx_buffer.take().unwrap_or(0),
            UART_LSR => {
                let mut lsr = 0x60;
                if self.rx_buffer.is_some() {
                    lsr |= 0x01;
                }
                lsr
            }
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            UART_RHR_THR => {
                // Local terminal output only for UART0 (console)
                if self.ipc.is_some() {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    let _ = handle.write_all(&[val]);
                    let _ = handle.flush();
                }

                // Forward to external UART device via IPC
                if let Some(ref mut ipc) = self.ipc {
                    ipc.send(&[val]);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uart_new() {
        let uart = Uart::new(None);
        assert!(uart.ipc.is_none());
    }

    #[test]
    fn test_uart_with_ipc() {
        let uart = Uart::new(Some("/tmp/test_uart.sock"));
        assert!(uart.ipc.is_some());
    }

    #[test]
    fn test_uart_read_write() {
        let mut uart = Uart::new(None);
        uart.write_u8(UART_RHR_THR, 0x41); // 'A'
        assert_eq!(uart.read_u8(UART_RHR_THR), 0);
        assert_eq!(uart.read_u8(UART_LSR), 0x60);
    }
}