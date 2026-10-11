use std::fs;
use std::path::Path;

pub const FLASH_BASE: u32 = 0x4000_0000;
pub const FLASH_SIZE: usize = 16 * 1024 * 1024; // 16 MB Flash Window

pub struct QspiFlash {
    pub data: Vec<u8>,
    backing_file: Option<String>,
    dirty: bool,
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

        Self { data, backing_file, dirty }
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

    /// Write a single byte to flash (for byte-programming emulation)
    pub fn write_u8(&mut self, offset: u32, val: u8) {
        let idx = offset as usize;
        if idx < self.data.len() {
            self.data[idx] = val;
            self.dirty = true;
        }
    }

    /// Write multiple bytes to flash
    pub fn write_bytes(&mut self, offset: u32, bytes: &[u8]) {
        let start = offset as usize;
        let end = (start + bytes.len()).min(self.data.len());
        if start < self.data.len() {
            self.data[start..end].copy_from_slice(&bytes[0..(end - start)]);
            self.dirty = true;
        }
    }

    /// Erase a sector (set to 0xFF)
    pub fn erase_sector(&mut self, sector: u32, sector_size: u32) {
        let start = (sector * sector_size) as usize;
        let end = ((sector + 1) * sector_size).min(self.data.len() as u32) as usize;
        if start < self.data.len() {
            for i in start..end {
                self.data[i] = 0xFF;
            }
            self.dirty = true;
        }
    }

    /// Erase a range of bytes
    pub fn erase_range(&mut self, offset: u32, len: u32) {
        let start = offset as usize;
        let end = (start + len as usize).min(self.data.len());
        if start < self.data.len() {
            for i in start..end {
                self.data[i] = 0xFF;
            }
            self.dirty = true;
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
}

impl Drop for QspiFlash {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}