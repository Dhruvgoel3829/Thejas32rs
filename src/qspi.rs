use std::fs;
use std::path::Path;

pub const FLASH_BASE: u32 = 0x4000_0000;
pub const FLASH_SIZE: usize = 16 * 1024 * 1024; // 16 MB Flash Window
/// Minimal controller doorbell window (CMD/ARG/STATUS, see ctrl_read/write)
pub const QSPI_CTRL_BASE: u32 = 0x100A_0000;

/// NOR flash geometry (matches typical SPI NOR, e.g. Winbond W25Q series)
pub const FLASH_SECTOR_SIZE: u32 = 4096; // 4 KB erase sectors
pub const FLASH_PAGE_SIZE: u32 = 256; // 256 B program pages

/// Busy durations in emulator cycles (nominal MTIME_HZ = 100 MHz).
/// Paired with firmware constants QSPI_PAGE_PROGRAM_US / QSPI_SECTOR_ERASE_MS
/// in firmware/include/config.h — keep both sides in sync.
pub const FLASH_PAGE_PROGRAM_CYCLES: u64 = 50_000; // ~0.5 ms @ 100 MHz
pub const FLASH_SECTOR_ERASE_CYCLES: u64 = 20_000_000; // ~200 ms @ 100 MHz

pub struct QspiFlash {
    pub data: Vec<u8>,
    backing_file: Option<String>,
    dirty: bool,
    busy_cycles: u64,
    cmd_arg: u32,
}

impl QspiFlash {
    pub fn new() -> Self {
        Self::with_backing_file(None)
    }

    pub fn with_backing_file(backing_file: Option<String>) -> Self {
        let mut data = vec![0xFF; FLASH_SIZE];
        let mut dirty = false;

        if let Some(ref path) = backing_file {
            if Path::new(path).exists() {
                match fs::read(path) {
                    Ok(file_data) if file_data.len() == FLASH_SIZE => {
                        data = file_data;
                        println!("[QSPI] Loaded flash from {}", path);
                    }
                    Ok(file_data) => {
                        // Wrong size, use default erased state
                        let copy_len = file_data.len().min(FLASH_SIZE);
                        data[..copy_len].copy_from_slice(&file_data[..copy_len]);
                        dirty = true; // Need to rewrite with correct size
                        println!("[QSPI] Loaded partial flash from {} ({} bytes)", path, file_data.len());
                    }
                    Err(e) => {
                        println!("[QSPI] Warning: Failed to load flash from {}: {}", path, e);
                    }
                }
            } else {
                dirty = true; // New file will be created on first save
            }
        }

        Self { data, backing_file, dirty, busy_cycles: 0, cmd_arg: 0 }
    }

    #[inline(always)]
    pub fn read_u8(&self, offset: u32) -> u8 {
        let idx = offset as usize;
        if idx < self.data.len() {
            self.data[idx]
        } else {
            0xFF
        }
    }

    #[inline(always)]
    pub fn read_u16(&self, offset: u32) -> u16 {
        let b0 = self.read_u8(offset) as u16;
        let b1 = self.read_u8(offset + 1) as u16;
        b0 | (b1 << 8)
    }

    #[inline(always)]
    pub fn read_u32(&self, offset: u32) -> u32 {
        let b0 = self.read_u8(offset) as u32;
        let b1 = self.read_u8(offset + 1) as u32;
        let b2 = self.read_u8(offset + 2) as u32;
        let b3 = self.read_u8(offset + 3) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Write a single byte to flash (NOR program: only clears bits).
    /// Single-byte MMIO programs are modeled as instant (no busy window);
    /// multi-byte programs and erases set busy (see write_bytes,
    /// erase_sector). Writes issued while busy are ignored.
    pub fn write_u8(&mut self, offset: u32, val: u8) {
        if self.busy_cycles > 0 {
            return;
        }
        let idx = offset as usize;
        if idx < self.data.len() {
            self.data[idx] &= val;
            self.dirty = true;
        }
    }

    /// Write multiple bytes to flash (one NOR page-program operation).
    /// Programs do not cross 256 B page boundaries: bytes past the end
    /// of the starting page wrap to the start of the SAME page
    /// (real NOR behavior for cross-page programs). Sets busy once for
    /// the whole call; calls issued while busy are dropped entirely.
    pub fn write_bytes(&mut self, offset: u32, bytes: &[u8]) {
        if self.busy_cycles > 0 {
            return;
        }
        let page_base = offset & !(FLASH_PAGE_SIZE - 1);
        let mut programmed = false;
        for (i, &b) in bytes.iter().enumerate() {
            let page_off = ((offset - page_base) + i as u32) % FLASH_PAGE_SIZE;
            let idx = (page_base + page_off) as usize;
            if idx < self.data.len() {
                self.data[idx] &= b;
                programmed = true;
            }
        }
        if programmed {
            self.dirty = true;
            self.busy_cycles = FLASH_PAGE_PROGRAM_CYCLES;
        }
    }

    /// Erase one fixed 4 KB sector (sets all bytes to 0xFF).
    pub fn erase_sector(&mut self, sector: u32) {
        if self.busy_cycles > 0 {
            return;
        }
        let start = (sector * FLASH_SECTOR_SIZE) as usize;
        let end = (start + FLASH_SECTOR_SIZE as usize).min(self.data.len());
        if start < self.data.len() {
            for i in start..end {
                self.data[i] = 0xFF;
            }
            self.dirty = true;
            self.busy_cycles = FLASH_SECTOR_ERASE_CYCLES;
        }
    }

    /// Erase a range of bytes (sets to 0xFF).
    /// Real NOR erases whole overlapped 4 KB sectors; busy time scales
    /// with the number of touched sectors (conservative).
    pub fn erase_range(&mut self, offset: u32, len: u32) {
        if self.busy_cycles > 0 {
            return;
        }
        let start = offset as usize;
        let end = (start + len as usize).min(self.data.len());
        if start < self.data.len() {
            for i in start..end {
                self.data[i] = 0xFF;
            }
            self.dirty = true;
            let sectors = ((len + FLASH_SECTOR_SIZE - 1) / FLASH_SECTOR_SIZE).max(1) as u64;
            self.busy_cycles = sectors * FLASH_SECTOR_ERASE_CYCLES;
        }
    }

    /// Load raw binary or section payload into Flash (for ELF loading)
    pub fn load_bytes(&mut self, offset: u32, bytes: &[u8]) {
        let start = offset as usize;
        let end = (start + bytes.len()).min(self.data.len());
        if start < self.data.len() {
            self.data[start..end].copy_from_slice(&bytes[0..(end - start)]);
            self.dirty = true;
        }
    }

    /// Persist flash to backing file if dirty
    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        if self.dirty {
            if let Some(ref path) = self.backing_file {
                fs::write(path, &self.data)?;
                self.dirty = false;
                println!("[QSPI] Saved flash to {}", path);
            }
        }
        Ok(())
    }

    /// Check if flash has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Set backing file path
    pub fn set_backing_file(&mut self, path: Option<String>) {
        self.backing_file = path;
    }

    /// Advance the internal busy counter by one emulator cycle.
    pub fn tick(&mut self) {
        if self.busy_cycles > 0 {
            self.busy_cycles -= 1;
        }
    }


    /// True while an erase or program operation is still busy.
    pub fn is_busy(&self) -> bool {
        self.busy_cycles > 0
    }

    /// Minimal controller doorbell (the ONLY control interface; all
    /// sequencing and timing policy lives in firmware hal_emu).
    /// CMD (offset 0x00): write 1 = erase the sector in ARG.
    /// ARG (offset 0x04): erase sector number (latched).
    /// STATUS (offset 0x08): bit 0 = busy (read-only).
    pub fn ctrl_write(&mut self, offset: u32, val: u32) {
        match offset {
            0x00 => {
                if val == 1 {
                    let sector = self.cmd_arg;
                    self.erase_sector(sector);
                }
            }
            0x04 => self.cmd_arg = val,
            _ => {}
        }
    }

    pub fn ctrl_read(&self, offset: u32) -> u32 {
        match offset {
            0x04 => self.cmd_arg,
            0x08 => self.is_busy() as u32,
            _ => 0,
        }
    }
}

impl Drop for QspiFlash {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_clears_bits_only() {
        let mut f = QspiFlash::new();
        // Fresh flash is erased (0xFF)
        assert_eq!(f.read_u8(0), 0xFF);
        // Program clears bits: 0xFF & 0x0F = 0x0F
        f.write_u8(0, 0x0F);
        // Tick past program busy so the next op is accepted
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0), 0x0F);
        // Programming 1s over 0s does nothing: 0x0F & 0xF0 = 0x00, then & 0xFF stays 0x00
        f.write_u8(0, 0xF0);
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0), 0x00);
        f.write_u8(0, 0xFF);
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0), 0x00); // bits stay cleared without erase
        // Erase restores 0xFF
        f.erase_sector(0);
        for _ in 0..FLASH_SECTOR_ERASE_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0), 0xFF);
    }

    #[test]
    fn test_writes_ignored_while_busy() {
        let mut f = QspiFlash::new();
        // Erase puts the controller busy for a sector-erase window
        f.erase_sector(0);
        assert!(f.is_busy());
        // Byte and multi-byte writes issued while busy are dropped
        f.write_u8(0, 0x00);
        f.write_bytes(1, &[0x00]);
        assert_eq!(f.read_u8(0), 0xFF);
        assert_eq!(f.read_u8(1), 0xFF);
        for _ in 0..FLASH_SECTOR_ERASE_CYCLES {
            f.tick();
        }
        assert!(!f.is_busy());
        // After busy clears, writes land
        f.write_u8(0, 0x00);
        assert_eq!(f.read_u8(0), 0x00);
    }

    #[test]
    fn test_write_bytes_sets_busy_once() {
        let mut f = QspiFlash::new();
        f.write_bytes(0, &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert!(f.is_busy());
        // Whole 4-byte op landed despite busy being set by the op itself
        assert_eq!(f.read_u8(0), 0xAA);
        assert_eq!(f.read_u8(3), 0xDD);
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert!(!f.is_busy());
    }

    #[test]
    fn test_page_boundary_wrap() {
        let mut f = QspiFlash::new();
        // Write 4 bytes straddling a 256 B page boundary (0xFF..0x102):
        // bytes at 0xFF, then wrap to 0x00, 0x01, 0x02 of the same page
        f.write_bytes(0xFF, &[0xAA, 0xBB, 0xCC, 0xDD]);
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0xFF), 0xAA);
        assert_eq!(f.read_u8(0x00), 0xBB); // wrapped, did not touch 0x100
        assert_eq!(f.read_u8(0x01), 0xCC);
        assert_eq!(f.read_u8(0x02), 0xDD);
        assert_eq!(f.read_u8(0x100), 0xFF);
    }

    #[test]
    fn test_erase_sector_fixed_4k() {
        let mut f = QspiFlash::new();
        f.write_u8(0x1000, 0x00);
        for _ in 0..FLASH_PAGE_PROGRAM_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0x1000), 0x00);
        f.erase_sector(1); // sector 1 = 0x1000..0x1FFF
        for _ in 0..FLASH_SECTOR_ERASE_CYCLES {
            f.tick();
        }
        assert_eq!(f.read_u8(0x1000), 0xFF);
        assert_eq!(f.read_u8(0x1FFF), 0xFF);
        // Adjacent sector untouched (still erased default)
        assert_eq!(f.read_u8(0x2000), 0xFF);
    }
}