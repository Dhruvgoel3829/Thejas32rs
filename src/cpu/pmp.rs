#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmpAccessType {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PmpMode {
    #[default]
    Off = 0,
    Tor = 1,
    Na4 = 2,
    Napot = 3,
}

impl From<u8> for PmpMode {
    fn from(val: u8) -> Self {
        match val & 0x3 {
            0 => PmpMode::Off,
            1 => PmpMode::Tor,
            2 => PmpMode::Na4,
            3 => PmpMode::Napot,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PmpConfig {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub mode: PmpMode,
    pub locked: bool,
}

impl PmpConfig {
    pub fn from_u8(byte: u8) -> Self {
        Self {
            read: (byte & 0x01) != 0,
            write: (byte & 0x02) != 0,
            exec: (byte & 0x04) != 0,
            mode: PmpMode::from((byte >> 3) & 0x3),
            locked: (byte & 0x80) != 0,
        }
    }

    pub fn to_u8(&self) -> u8 {
        (self.read as u8)
            | ((self.write as u8) << 1)
            | ((self.exec as u8) << 2)
            | ((self.mode as u8) << 3)
            | ((self.locked as u8) << 7)
    }
}

#[derive(Default)]
pub struct Pmp {
    pub cfg: [PmpConfig; 16],
    pub addr: [u32; 16],
}

impl Pmp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_cfg(&self, reg_idx: usize) -> u32 {
        let base = reg_idx * 4;
        let b0 = self.cfg[base].to_u8() as u32;
        let b1 = self.cfg[base + 1].to_u8() as u32;
        let b2 = self.cfg[base + 2].to_u8() as u32;
        let b3 = self.cfg[base + 3].to_u8() as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write_cfg(&mut self, reg_idx: usize, val: u32) {
        let base = reg_idx * 4;
        let bytes = val.to_le_bytes();
        for i in 0..4 {
            let entry = base + i;
            if !self.cfg[entry].locked {
                self.cfg[entry] = PmpConfig::from_u8(bytes[i]);
            }
        }
    }

    pub fn read_addr(&self, idx: usize) -> u32 {
        if idx < 16 {
            self.addr[idx]
        } else {
            0
        }
    }

    pub fn write_addr(&mut self, idx: usize, val: u32) {
        if idx < 16 && !self.cfg[idx].locked {
            self.addr[idx] = val;
        }
    }

    fn get_range(&self, i: usize) -> (u32, u32) {
        match self.cfg[i].mode {
            PmpMode::Off => (0, 0),
            PmpMode::Tor => {
                let start = if i == 0 { 0 } else { self.addr[i - 1] << 2 };
                let end = self.addr[i] << 2;
                (start, end)
            }
            PmpMode::Na4 => {
                let start = self.addr[i] << 2;
                (start, start.wrapping_add(4))
            }
            PmpMode::Napot => {
                let a = self.addr[i];
                let trailing_ones = (!a).trailing_zeros();
                let size = 1u64 << (trailing_ones + 3);
                let base = ((a as u64 & !((1u64 << trailing_ones) - 1)) << 2) as u32;
                (base, base.wrapping_add(size as u32))
            }
        }
    }

    pub fn check_access(
        &self,
        addr: u32,
        len: u32,
        access: PmpAccessType,
        priv_mode: u8,
    ) -> Result<(), ()> {
        let end_addr = addr.wrapping_add(len);

        for i in 0..16 {
            if self.cfg[i].mode == PmpMode::Off {
                continue;
            }

            let (range_start, range_end) = self.get_range(i);

            if addr >= range_start && end_addr <= range_end {
                // In Machine mode, unlocked entries are unconditionally allowed
                if priv_mode == 3 && !self.cfg[i].locked {
                    return Ok(());
                }

                let allowed = match access {
                    PmpAccessType::Read => self.cfg[i].read,
                    PmpAccessType::Write => self.cfg[i].write,
                    PmpAccessType::Execute => self.cfg[i].exec,
                };

                return if allowed { Ok(()) } else { Err(()) };
            }
        }

        if priv_mode == 3 {
            Ok(())
        } else {
            Err(())
        }
    }
}