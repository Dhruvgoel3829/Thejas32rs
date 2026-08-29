use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const UART_RHR_THR: u32 = 0x00;
const UART_LSR: u32 = 0x05;

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x1000;

pub struct Uart {
    rx_receiver: Receiver<u8>,
    rx_buffer: Option<u8>,
}

impl Uart {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        // Background worker thread for polling raw terminal events
        thread::spawn(move || {
            loop {
                // Poll every 10ms so we don't pin the host CPU thread
                if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        // Only handle Key Press, ignore Release / Repeat
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
                                KeyCode::Backspace => Some(0x7F), // Standard DEL/Backspace
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

        Self {
            rx_receiver: rx,
            rx_buffer: None,
        }
    }

    pub fn read_u8(&mut self, offset: u32) -> u8 {
        if self.rx_buffer.is_none() {
            if let Ok(byte) = self.rx_receiver.try_recv() {
                self.rx_buffer = Some(byte);
            }
        }

        match offset {
            UART_RHR_THR => self.rx_buffer.take().unwrap_or(0),
            UART_LSR => {
                let mut lsr = 0x60; // TX always ready
                if self.rx_buffer.is_some() {
                    lsr |= 0x01; // Data ready bit
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
