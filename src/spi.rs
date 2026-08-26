pub const SPI_BASE: u32 = 0x1003_0000;
pub const SPI_SIZE: u32 = 0x1000;

// Register Offsets
const SPI_CR: u32   = 0x00; // Control Register (bit 0: enable, bit 1: loopback)
const SPI_SR: u32   = 0x04; // Status Register (bit 0: TX ready, bit 1: RX ready)
const SPI_DATA: u32 = 0x08; // TX/RX Data Register

pub struct Spi {
    pub cr: u32,
    pub rx_fifo: Option<u8>,
}

impl Spi {
    pub fn new() -> Self {
        Self {
            cr: 0,
            rx_fifo: None,
        }
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            SPI_CR => self.cr,
            SPI_SR => {
                let mut sr = 0x01; // TX always ready
                if self.rx_fifo.is_some() {
                    sr |= 0x02; // RX byte ready
                }
                sr
            }
            SPI_DATA => self.rx_fifo.take().unwrap_or(0) as u32,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            SPI_CR => self.cr = val,
            SPI_DATA => {
                let byte_out = (val & 0xFF) as u8;
                let is_loopback = (self.cr & 0x02) != 0;

                if is_loopback {
                    // Loopback mode: immediately send back the written byte
                    self.rx_fifo = Some(byte_out);
                } else {
                    // Virtual SPI Flash Mock: returns dummy response (e.g. JEDEC ID 0xEF for Winbond)
                    self.rx_fifo = match byte_out {
                        0x9F => Some(0xEF), // JEDEC ID query
                        _ => Some(byte_out.wrapping_add(1)),
                    };
                }
            }
            _ => {}
        }
    }
}