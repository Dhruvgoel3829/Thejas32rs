use crate::gazebo_backend::GazeboImuBackend;

pub const SPI_BASE: u32 = 0x1003_0000;
pub const SPI_SIZE: u32 = 0x1000;

const SPI_CTRL_OFFSET: u32 = 0x00;
const SPI_DATA_OFFSET: u32 = 0x04;
const SPI_STATUS_OFFSET: u32 = 0x08;
const SPI_CS_OFFSET: u32 = 0x0C;

pub struct Spi {
    pub ctrl: u32,
    pub data_rx: u8,
    pub status: u32,
    pub cs: u32,
    gazebo: Option<GazeboImuBackend>,
}

impl Spi {
    pub fn new(gazebo: bool) -> Self {
        Self {
            ctrl: 0,
            data_rx: 0xFF,
            status: 0x01,
            cs: 0xFFFF_FFFF,
            gazebo: if gazebo {
                Some(GazeboImuBackend::new().expect("Failed to create Gazebo IMU backend"))
            } else {
                None
            },
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
            }
            SPI_DATA_OFFSET => {
                let cs_state = (self.cs & 0xFF) as u8;
                let mosi = (val & 0xFF) as u8;
                if let Some(ref mut gazebo) = self.gazebo {
                    self.data_rx = gazebo.spi_exchange(cs_state, mosi);
                } else {
                    self.data_rx = 0xFF;
                }
                self.status |= 0x01;
            }
            _ => {}
        }
    }
}