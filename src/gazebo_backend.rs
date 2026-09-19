use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;

pub const GAZEBO_IMU_PORT: u16 = 9002;
pub const GAZEBO_MOTOR_PORT: u16 = 9003;
pub const GAZEBO_TELEMETRY_PORT: u16 = 9004;
pub const GAZEBO_ADDR: &str = "127.0.0.1";

pub struct GazeboImuBackend {
    socket: UdpSocket,
    server_addr: SocketAddr,
    imu_buffer: [u8; 56],
    buffer_valid: bool,
    spi_index: usize,
    cs_asserted: bool,
}

impl GazeboImuBackend {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind((GAZEBO_ADDR, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(1)))?;
        socket.set_nonblocking(true)?;
        let server_addr: SocketAddr = format!("{}:{}", GAZEBO_ADDR, GAZEBO_IMU_PORT).parse().unwrap();
        Ok(Self {
            socket,
            server_addr,
            imu_buffer: [0; 56],
            buffer_valid: false,
            spi_index: 0,
            cs_asserted: false,
        })
    }

    fn fetch_imu(&mut self) {
        let mut buf = [0u8; 56];
        match self.socket.recv_from(&mut buf) {
            Ok((56, _)) => {
                self.imu_buffer = buf;
                self.buffer_valid = true;
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }
    }

    pub fn spi_exchange(&mut self, cs_state: u8, mosi: u8) -> u8 {
        let cs_active = (cs_state & 0x01) == 0;

        if !self.cs_asserted && cs_active {
            self.fetch_imu();
            self.spi_index = 0;
        }
        self.cs_asserted = cs_active;

        if !cs_active {
            return 0xFF;
        }

        if !self.buffer_valid {
            return 0x00;
        }

        let reg = (mosi & 0x7F) as usize;
        let is_read = (mosi & 0x80) != 0;

        if self.spi_index == 0 && is_read {
            self.spi_index += 1;
            return 0x00;
        }

        if is_read && reg >= 0x3B && reg <= 0x48 && self.spi_index < 15 {
            let idx = reg - 0x3B;
            let val = self.imu_buffer[8 + idx];
            self.spi_index += 1;
            val
        } else if is_read && reg == 0x75 {
            0x68
        } else {
            0x00
        }
    }
}

pub struct GazeboMotorBackend {
    socket: UdpSocket,
    server_addr: SocketAddr,
    motor_cmd: [f32; 4],
}

impl GazeboMotorBackend {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind((GAZEBO_ADDR, 0))?;
        socket.set_nonblocking(true)?;
        let server_addr: SocketAddr = format!("{}:{}", GAZEBO_ADDR, GAZEBO_MOTOR_PORT).parse().unwrap();
        Ok(Self {
            socket,
            server_addr,
            motor_cmd: [0.0, 0.0, 0.0, 0.0],
        })
    }

    pub fn send_motors(&mut self, data: u32) {
        let bytes = data.to_le_bytes();
        for i in 0..4 {
            self.motor_cmd[i] = bytes[i] as f32 / 255.0;
        }
        let buf: [f32; 4] = self.motor_cmd;
        let bytes: [u8; 16] = unsafe { std::mem::transmute(buf) };
        let _ = self.socket.send_to(&bytes, self.server_addr);
    }
}

pub struct GazeboUartBackend {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl GazeboUartBackend {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind((GAZEBO_ADDR, 0))?;
        socket.set_nonblocking(true)?;
        let server_addr: SocketAddr = format!("{}:{}", GAZEBO_ADDR, GAZEBO_TELEMETRY_PORT).parse().unwrap();
        Ok(Self { socket, server_addr })
    }

    pub fn write(&mut self, byte: u8) {
        let _ = self.socket.send_to(&[byte], self.server_addr);
        print!("{}", byte as char);
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
}