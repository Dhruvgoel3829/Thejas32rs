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
const UART_DLL: u32 = 0x00;  // Divisor Latch Low
const UART_DLM: u32 = 0x01;  // Divisor Latch High
const UART_IER: u32 = 0x01;  // Interrupt Enable Register
const UART_FCR: u32 = 0x02;  // FIFO Control Register
const UART_LCR: u32 = 0x03;  // Line Control Register
const UART_MCR: u32 = 0x04;  // Modem Control Register
const UART_SCR: u32 = 0x07;  // Scratch Register

// LSR bits
const UART_LSR_DR: u8 = 0x01;   // Data Ready
const UART_LSR_OE: u8 = 0x02;   // Overrun Error
const UART_LSR_PE: u8 = 0x04;   // Parity Error
const UART_LSR_FE: u8 = 0x08;   // Framing Error
const UART_LSR_BI: u8 = 0x10;   // Break Interrupt
const UART_LSR_THRE: u8 = 0x20; // Transmitter Holding Register Empty
const UART_LSR_TEMT: u8 = 0x40; // Transmitter Empty
const UART_LSR_FIFOE: u8 = 0x80; // FIFO Error

// LCR bits
const UART_LCR_DLAB: u8 = 0x80; // Divisor Latch Access Bit

// Default baud rate
const DEFAULT_BAUD: u32 = 115200;
const UART_CLK_FREQ: u32 = 1_843_200; // Standard UART clock frequency (1.8432 MHz)

pub struct Uart {
    rx_receiver: Receiver<u8>,
    rx_buffer: Option<u8>,
    ipc: Option<SocketBackend>,
    console: bool,
    baud_rate: u32,
    dll: u8,  // Divisor Latch Low
    dlm: u8,  // Divisor Latch High
    lcr: u8,  // Line Control Register
    tx_busy_cycles: u64,  // Cycles remaining for current TX
    tx_byte: Option<u8>,  // Byte being transmitted
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
            baud_rate: DEFAULT_BAUD,
            dll: 0x01,  // Default divisor for 115200 @ 1.8432 MHz
            dlm: 0x00,
            lcr: 0x03,  // 8 data bits, 1 stop bit, no parity
            tx_busy_cycles: 0,
            tx_byte: None,
            console: false,
        }
    }

    /// Mark this UART as the console (UART0): TX bytes mirror to stdout.
    pub fn set_console(&mut self, console: bool) {
        self.console = console;
    }

    fn divisor(&self) -> u16 {
        ((self.dlm as u16) << 8) | (self.dll as u16)
    }

    fn bit_time_cycles(&self) -> u64 {
        // Cycles per bit = MTIME_HZ / baud_rate
        // MTIME_HZ is typically 100_000_000 (100 MHz)
        100_000_000 / self.baud_rate as u64
    }

    fn byte_time_cycles(&self) -> u64 {
        // 10 bits per byte (1 start + 8 data + 1 stop)
        self.bit_time_cycles() * 10
    }

    pub fn tick(&mut self) {
        if self.tx_busy_cycles > 0 {
            self.tx_busy_cycles -= 1;
            if self.tx_busy_cycles == 0 {
                self.tx_byte = None;
            }
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
                let mut lsr = UART_LSR_THRE | UART_LSR_TEMT;  // TX ready by default
                if self.rx_buffer.is_some() {
                    lsr |= UART_LSR_DR;
                }
                if self.tx_busy_cycles > 0 {
                    lsr &= !UART_LSR_THRE;  // TX not ready if busy
                    lsr &= !UART_LSR_TEMT;
                }
                lsr
            }
            UART_DLL => self.dll,
            UART_DLM => self.dlm,
            UART_LCR => self.lcr,
            _ => 0,
        }
    }

    pub fn write_u8(&mut self, offset: u32, val: u8) {
        match offset {
            UART_RHR_THR => {
                if self.tx_busy_cycles > 0 {
                    // TX buffer full, ignore or queue (for now, ignore)
                    return;
                }
                
                // Start transmission
                self.tx_byte = Some(val);
                self.tx_busy_cycles = self.byte_time_cycles();
                
                // Console UART (UART0) mirrors TX to stdout so firmware
                // printfs are visible with or without --ipc
                if self.console {
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
            UART_IER => {
                // Interrupt Enable Register - ignore for now
            }
            UART_FCR => {
                // FIFO Control Register - ignore for now
            }
            UART_LCR => {
                self.lcr = val;
            }
            UART_DLL => {
                if (self.lcr & UART_LCR_DLAB) != 0 {
                    self.dll = val;
                    self.update_baud_rate();
                }
            }
            UART_DLM => {
                if (self.lcr & UART_LCR_DLAB) != 0 {
                    self.dlm = val;
                    self.update_baud_rate();
                }
            }
            _ => {}
        }
    }

    fn update_baud_rate(&mut self) {
        let divisor = self.divisor() as u32;
        if divisor > 0 {
            self.baud_rate = UART_CLK_FREQ / (16 * divisor as u32);
        } else {
            self.baud_rate = DEFAULT_BAUD;
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
        // Idle: TX holding register empty, transmitter empty, no RX data
        assert_eq!(uart.read_u8(UART_LSR), 0x60);
        uart.write_u8(UART_RHR_THR, 0x41); // 'A' -> starts transmission
        // Immediately after write: TX busy, THRE/TEMT cleared, no RX data
        assert_eq!(uart.read_u8(UART_LSR), 0x00);
        assert_eq!(uart.read_u8(UART_RHR_THR), 0);
        // Tick past one full byte time (10 bits): TX completes
        let cycles = uart.byte_time_cycles();
        for _ in 0..cycles {
            uart.tick();
        }
        assert_eq!(uart.read_u8(UART_LSR), 0x60);
    }

    #[test]
    fn test_baud_rate() {
        let uart = Uart::new(None);
        assert_eq!(uart.baud_rate, DEFAULT_BAUD);
        assert_eq!(uart.divisor(), 1);
    }
}