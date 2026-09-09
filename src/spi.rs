// src/spi.rs
use crate::ipc::SocketBackend;

pub const SPI_BASE: u32 = 0x1003_0000;
pub const SPI_SIZE: u32 = 0x1000;

const SPI_CTRL_OFFSET: u32   = 0x00;
const SPI_DATA_OFFSET: u32   = 0x04;
const SPI_STATUS_OFFSET: u32 = 0x08;
const SPI_CS_OFFSET: u32     = 0x0C;

pub struct Spi {
    pub ctrl: u32,
    pub data_rx: u8,
    pub status: u32,
    pub cs: u32,
    pub ipc: SocketBackend,
}

impl Spi {
    pub fn new() -> Self {
        Self {
            ctrl: 0,
            data_rx: 0xFF,
            status: 0x01, // TX ready
            cs: 0xFFFF_FFFF,
            ipc: SocketBackend::new("/tmp/thejas_spi.sock"),
        }
    }

    pub fn read_u32(&self, offset: u32) -> u32 {
        match offset {
            SPI_CTRL_OFFSET => self.ctrl,
            SPI_DATA_OFFSET => self.data_rx as u32,
            SPI_STATUS_OFFSET => self.status,
            SPI_CS_OFFSET => self.cs,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, val: u32) {
        match offset {
            SPI_CTRL_OFFSET => self.ctrl = val,
            SPI_CS_OFFSET => {
                self.cs = val;
                // Notify external proxy of CS assertion/deassertion (Packet Type 0x01)
                let cs_byte = (val & 0xFF) as u8;
                self.ipc.send(&[0x01, cs_byte]);
            }
            SPI_DATA_OFFSET => {
                let mosi = (val & 0xFF) as u8;
                let cs_state = (self.cs & 0xFF) as u8;
                // Stream transfer over socket and read MISO
                self.data_rx = self.ipc.spi_exchange(cs_state, mosi);
                self.status |= 0x01; // RX data ready
            }
            _ => {}
        }
    }
}