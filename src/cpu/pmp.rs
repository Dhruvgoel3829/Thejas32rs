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
                let trailing_zeros = a.trailing_zeros();
                let size = 1u64 << (trailing_zeros + 3);
                let base = ((a as u64 & !((1u64 << trailing_zeros) - 1)) << 2) as u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmp_new() {
        let pmp = Pmp::new();
        for i in 0..16 {
            assert_eq!(pmp.cfg[i].read, false);
            assert_eq!(pmp.cfg[i].write, false);
            assert_eq!(pmp.cfg[i].exec, false);
            assert_eq!(pmp.cfg[i].mode, PmpMode::Off);
            assert_eq!(pmp.cfg[i].locked, false);
            assert_eq!(pmp.addr[i], 0);
        }
    }

    #[test]
    fn test_pmp_config_from_to_u8() {
        let cfg = PmpConfig {
            read: true,
            write: true,
            exec: true,
            mode: PmpMode::Napot,
            locked: true,
        };
        let byte = cfg.to_u8();
        assert_eq!(byte & 0x01, 0x01);
        assert_eq!(byte & 0x02, 0x02);
        assert_eq!(byte & 0x04, 0x04);
        assert_eq!((byte >> 3) & 0x3, 0x3);
        assert_eq!(byte & 0x80, 0x80);

        let cfg2 = PmpConfig::from_u8(byte);
        assert_eq!(cfg2.read, true);
        assert_eq!(cfg2.write, true);
        assert_eq!(cfg2.exec, true);
        assert_eq!(cfg2.mode, PmpMode::Napot);
        assert_eq!(cfg2.locked, true);
    }

    #[test]
    fn test_pmp_mode_from_u8() {
        assert_eq!(PmpMode::from(0), PmpMode::Off);
        assert_eq!(PmpMode::from(1), PmpMode::Tor);
        assert_eq!(PmpMode::from(2), PmpMode::Na4);
        assert_eq!(PmpMode::from(3), PmpMode::Napot);
    }

#[test]
    fn test_pmp_read_write_cfg() {
        let mut pmp = Pmp::new();
        // Valid PMP config bytes (no reserved bits set):
        // cfg[0]: R=1,W=1,X=1,mode=Napot(3),L=0 -> 0x0F
        // cfg[1]: R=1,W=1,X=0,mode=Tor(1),L=0 -> 0x0B
        // cfg[2]: R=1,W=0,X=1,mode=Na4(2),L=0 -> 0x15
        // cfg[3]: R=0,W=0,X=0,mode=Off(0),L=1 -> 0x80
        let val = u32::from_le_bytes([0x0F, 0x0B, 0x15, 0x80]);
        pmp.write_cfg(0, val);
        let read_val = pmp.read_cfg(0);
        assert_eq!(read_val, val);
        
        assert_eq!(pmp.cfg[0].to_u8(), 0x0F);
        assert_eq!(pmp.cfg[1].to_u8(), 0x0B);
        assert_eq!(pmp.cfg[2].to_u8(), 0x15);
        assert_eq!(pmp.cfg[3].to_u8(), 0x80);
    }

    #[test]
    fn test_pmp_locked_config_not_writable() {
        let mut pmp = Pmp::new();
        pmp.cfg[0] = PmpConfig {
            read: true,
            write: false,
            exec: false,
            mode: PmpMode::Tor,
            locked: true,
        };
        pmp.write_cfg(0, 0x00_00_00_FF);
        assert_eq!(pmp.cfg[0].locked, true);
        assert_eq!(pmp.cfg[0].read, true);
        assert_eq!(pmp.cfg[1].to_u8(), 0x00);
    }

    #[test]
    fn test_pmp_read_write_addr() {
        let mut pmp = Pmp::new();
        pmp.write_addr(0, 0x1000);
        assert_eq!(pmp.read_addr(0), 0x1000);
        pmp.write_addr(15, 0x8000_0000);
        assert_eq!(pmp.read_addr(15), 0x8000_0000);
        assert_eq!(pmp.read_addr(16), 0);
    }

    #[test]
    fn test_pmp_locked_addr_not_writable() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].locked = true;
        pmp.write_addr(0, 0x1000);
        assert_eq!(pmp.read_addr(0), 0);
        pmp.write_addr(1, 0x2000);
        assert_eq!(pmp.read_addr(1), 0x2000);
    }

    #[test]
    fn test_pmp_get_range_tor() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Tor;
        pmp.addr[0] = 0x1000;
        let (start, end) = pmp.get_range(0);
        assert_eq!(start, 0);
        assert_eq!(end, 0x4000);
        pmp.cfg[1].mode = PmpMode::Tor;
        pmp.addr[1] = 0x2000;
        let (start, end) = pmp.get_range(1);
        assert_eq!(start, 0x4000);
        assert_eq!(end, 0x8000);
    }

    #[test]
    fn test_pmp_get_range_na4() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Na4;
        pmp.addr[0] = 0x1000;
        let (start, end) = pmp.get_range(0);
        assert_eq!(start, 0x4000);
        assert_eq!(end, 0x4004);
    }

    #[test]
    fn test_pmp_get_range_napot() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Napot;
        pmp.addr[0] = 0x1000;
        let (start, end) = pmp.get_range(0);
        assert_eq!(start, 0x4000);
        assert_eq!(end, 0xC000);
    }

    #[test]
    fn test_pmp_check_access_off() {
        let pmp = Pmp::new();
        assert!(pmp.check_access(0x1000, 4, PmpAccessType::Read, 3).is_ok());
        assert!(pmp.check_access(0x1000, 4, PmpAccessType::Write, 3).is_ok());
        assert!(pmp.check_access(0x1000, 4, PmpAccessType::Execute, 3).is_ok());
    }

    #[test]
    fn test_pmp_check_access_m_mode_unlocked() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Tor;
        pmp.cfg[0].read = false;
        pmp.cfg[0].write = false;
        pmp.cfg[0].exec = false;
        pmp.cfg[0].locked = false;
        pmp.addr[0] = 0x1000;
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Read, 3).is_ok());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Write, 3).is_ok());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Execute, 3).is_ok());
    }

    #[test]
    fn test_pmp_check_access_m_mode_locked() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Tor;
        pmp.cfg[0].read = true;
        pmp.cfg[0].write = false;
        pmp.cfg[0].exec = false;
        pmp.cfg[0].locked = true;
        pmp.addr[0] = 0x1000;
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Read, 3).is_ok());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Write, 3).is_err());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Execute, 3).is_err());
    }

    #[test]
    fn test_pmp_check_access_u_mode() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Tor;
        pmp.cfg[0].read = true;
        pmp.cfg[0].write = true;
        pmp.cfg[0].exec = true;
        pmp.cfg[0].locked = false;
        pmp.addr[0] = 0x1000;
        assert!(pmp.check_access(0x8000, 4, PmpAccessType::Read, 0).is_err());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Read, 0).is_ok());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Write, 0).is_ok());
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Execute, 0).is_ok());
        pmp.cfg[0].write = false;
        assert!(pmp.check_access(0x2000, 4, PmpAccessType::Write, 0).is_err());
    }

    #[test]
    fn test_pmp_check_access_napot() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Napot;
        pmp.cfg[0].read = true;
        pmp.cfg[0].write = true;
        pmp.cfg[0].exec = true;
        pmp.cfg[0].locked = true;
        pmp.addr[0] = 0x1000;
        assert!(pmp.check_access(0x5000, 4, PmpAccessType::Read, 0).is_ok());
        assert!(pmp.check_access(0xB000, 4, PmpAccessType::Write, 0).is_ok());
        assert!(pmp.check_access(0x3000, 4, PmpAccessType::Read, 0).is_err());
        assert!(pmp.check_access(0xC000, 4, PmpAccessType::Read, 0).is_err());
    }

    #[test]
    fn test_pmp_check_access_crosses_boundary() {
        let mut pmp = Pmp::new();
        pmp.cfg[0].mode = PmpMode::Tor;
        pmp.cfg[0].read = true;
        pmp.cfg[0].write = true;
        pmp.cfg[0].exec = true;
        pmp.cfg[0].locked = true;
        pmp.addr[0] = 0x1000;
        // Range is [0, 0x4000)
        // 0x3FFC + 4 = 0x4000 (end of range, valid - covers 0x3FFC..0x3FFF)
        assert!(pmp.check_access(0x3FFC, 4, PmpAccessType::Read, 0).is_ok());
        // 0x3FFD + 4 = 0x4001 (crosses boundary, invalid)
        assert!(pmp.check_access(0x3FFD, 4, PmpAccessType::Read, 0).is_err());
        // 0x3FF8 + 4 = 0x3FFC (well within range)
        assert!(pmp.check_access(0x3FF8, 4, PmpAccessType::Read, 0).is_ok());
    }
}