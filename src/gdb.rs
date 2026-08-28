// src/gdb.rs
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use crate::cpu::Cpu;

pub struct GdbStub {
    stream: TcpStream,
    breakpoints: HashSet<u32>,
}

impl GdbStub {
    pub fn wait_for_connection(port: u16) -> Self {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .expect("Failed to bind GDB TCP listener");
        eprintln!("[GDB] Listening on 127.0.0.1:{} (Run 'target remote :{}' in GDB)...", port, port);

        let (stream, addr) = listener.accept().expect("Failed to accept GDB connection");
        eprintln!("[GDB] Connected from {}", addr);

        Self {
            stream,
            breakpoints: HashSet::new(),
        }
    }

    fn read_packet(&mut self) -> Option<String> {
        let mut buf = [0u8; 1];
        // Wait for start of packet '$' or GDB interrupt (0x03)
        loop {
            if self.stream.read_exact(&mut buf).is_err() {
                return None;
            }
            if buf[0] == b'$' {
                break;
            } else if buf[0] == 0x03 {
                return Some("\x03".to_string());
            }
        }

        let mut payload = Vec::new();
        while self.stream.read_exact(&mut buf).is_ok() {
            if buf[0] == b'#' {
                break;
            }
            payload.push(buf[0]);
        }

        // Read 2-byte hex checksum
        let mut cksum = [0u8; 2];
        if self.stream.read_exact(&mut cksum).is_err() {
            return None;
        }

        // Send ACK
        let _ = self.stream.write_all(b"+");

        String::from_utf8(payload).ok()
    }

    fn send_packet(&mut self, payload: &str) {
        let cksum: u8 = payload.bytes().fold(0, |acc, b| acc.wrapping_add(b));
        let packet = format!("${}#{:02x}", payload, cksum);
        let _ = self.stream.write_all(packet.as_bytes());
    }

    /// Handles GDB interaction. Returns `true` when CPU should continue running, `false` to exit.
    pub fn handle_client(&mut self, cpu: &mut Cpu) -> bool {
        loop {
            let pkt = match self.read_packet() {
                Some(p) => p,
                None => return false,
            };

            if pkt.is_empty() {
                self.send_packet("");
                continue;
            }

            if pkt.starts_with("qSupported") {
                self.send_packet("PacketSize=1000;hwbreak+;vContSupported-");
            } else if pkt == "?" {
                self.send_packet("S05"); // TRAP signal 5 (SIGTRAP)
            } else if pkt == "g" {
                // Read all registers: x0..x31 + PC
                let mut s = String::new();
                for reg in cpu.regs.iter() {
                    s.push_str(&Self::u32_to_hex_le(*reg));
                }
                s.push_str(&Self::u32_to_hex_le(cpu.pc));
                self.send_packet(&s);
            } else if pkt.starts_with('G') {
                // Write all registers
                let data = &pkt[1..];
                for i in 0..32 {
                    if data.len() >= (i + 1) * 8 {
                        let hex = &data[i * 8..(i + 1) * 8];
                        if let Ok(val) = Self::hex_le_to_u32(hex) {
                            cpu.write_reg(i, val);
                        }
                    }
                }
                if data.len() >= 33 * 8 {
                    let hex = &data[32 * 8..33 * 8];
                    if let Ok(val) = Self::hex_le_to_u32(hex) {
                        cpu.pc = val;
                    }
                }
                self.send_packet("OK");
            } else if pkt.starts_with('p') {
                // Read single register
                let reg_idx = usize::from_str_radix(&pkt[1..], 16).unwrap_or(0);
                let val = if reg_idx < 32 {
                    cpu.read_reg(reg_idx)
                } else if reg_idx == 32 {
                    cpu.pc
                } else {
                    0
                };
                self.send_packet(&Self::u32_to_hex_le(val));
            } else if pkt.starts_with('P') {
                // Write single register
                let parts: Vec<&str> = pkt[1..].split('=').collect();
                if parts.len() == 2 {
                    let reg_idx = usize::from_str_radix(parts[0], 16).unwrap_or(0);
                    let val = Self::hex_le_to_u32(parts[1]).unwrap_or(0);
                    if reg_idx < 32 {
                        cpu.write_reg(reg_idx, val);
                    } else if reg_idx == 32 {
                        cpu.pc = val;
                    }
                }
                self.send_packet("OK");
            } else if pkt.starts_with('m') {
                // Read memory: m<addr>,<len>
                let parts: Vec<&str> = pkt[1..].split(',').collect();
                if parts.len() == 2 {
                    let addr = u32::from_str_radix(parts[0], 16).unwrap_or(0);
                    let len = usize::from_str_radix(parts[1], 16).unwrap_or(0);
                    let mut hex_data = String::new();
                    for i in 0..len {
                        let byte = cpu.bus.read_u8(addr.wrapping_add(i as u32));
                        hex_data.push_str(&format!("{:02x}", byte));
                    }
                    self.send_packet(&hex_data);
                } else {
                    self.send_packet("E01");
                }
            } else if pkt.starts_with('M') {
                // Write memory: M<addr>,<len>:<data>
                let header_and_data: Vec<&str> = pkt[1..].split(':').collect();
                if header_and_data.len() == 2 {
                    let meta: Vec<&str> = header_and_data[0].split(',').collect();
                    let addr = u32::from_str_radix(meta[0], 16).unwrap_or(0);
                    let len = usize::from_str_radix(meta[1], 16).unwrap_or(0);
                    let hex_bytes = header_and_data[1];
                    for i in 0..len {
                        if i * 2 + 2 <= hex_bytes.len() {
                            if let Ok(byte) = u8::from_str_radix(&hex_bytes[i * 2..i * 2 + 2], 16) {
                                cpu.bus.write_u8(addr.wrapping_add(i as u32), byte);
                            }
                        }
                    }
                    self.send_packet("OK");
                } else {
                    self.send_packet("E01");
                }
            } else if pkt.starts_with("Z0,") {
                // Insert breakpoint
                let parts: Vec<&str> = pkt.split(',').collect();
                if parts.len() >= 2 {
                    let addr = u32::from_str_radix(parts[1], 16).unwrap_or(0);
                    self.breakpoints.insert(addr);
                    self.send_packet("OK");
                } else {
                    self.send_packet("E01");
                }
            } else if pkt.starts_with("z0,") {
                // Remove breakpoint
                let parts: Vec<&str> = pkt.split(',').collect();
                if parts.len() >= 2 {
                    let addr = u32::from_str_radix(parts[1], 16).unwrap_or(0);
                    self.breakpoints.remove(&addr);
                    self.send_packet("OK");
                } else {
                    self.send_packet("E01");
                }
            } else if pkt == "s" {
                // Single step
                cpu.step(false);
                self.send_packet("S05");
            } else if pkt == "c" {
                // Continue
                return true;
            } else if pkt == "D" || pkt == "k" {
                // Detach / Kill
                self.send_packet("OK");
                return false;
            } else {
                // Unsupported packet
                self.send_packet("");
            }
        }
    }

    pub fn is_at_breakpoint(&self, pc: u32) -> bool {
        self.breakpoints.contains(&pc)
    }

    pub fn notify_stop(&mut self) {
        self.send_packet("S05");
    }

    fn u32_to_hex_le(val: u32) -> String {
        let b = val.to_le_bytes();
        format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
    }

    fn hex_le_to_u32(s: &str) -> Result<u32, ()> {
        if s.len() < 8 {
            return Err(());
        }
        let b0 = u8::from_str_radix(&s[0..2], 16).map_err(|_| ())?;
        let b1 = u8::from_str_radix(&s[2..4], 16).map_err(|_| ())?;
        let b2 = u8::from_str_radix(&s[4..6], 16).map_err(|_| ())?;
        let b3 = u8::from_str_radix(&s[6..8], 16).map_err(|_| ())?;
        Ok(u32::from_le_bytes([b0, b1, b2, b3]))
    }
}