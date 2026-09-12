use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

pub struct SocketBackend {
    listener: Option<UnixListener>,
    pub stream: Option<UnixStream>,
    path: String,
}

impl SocketBackend {
    pub fn new(path: &str) -> Self {
        // Clean up stale socket file if it exists
        if Path::new(path).exists() {
            let _ = fs::remove_file(path);
        }

        let listener = UnixListener::bind(path).ok();
        if let Some(ref l) = listener {
            let _ = l.set_nonblocking(true);
        }

        Self {
            listener,
            stream: None,
            path: path.to_string(),
        }
    }

    pub fn wait_for_client(&mut self) {
        if self.stream.is_some() {
            return;
        }

        println!("[IPC] Waiting for external sensor/simulator on {}...", self.path);
        if let Some(ref listener) = self.listener {
            // Temporarily set listener to blocking mode
            let _ = listener.set_nonblocking(false);
            if let Ok((stream, _)) = listener.accept() {
                let _ = stream.set_nonblocking(true);
                self.stream = Some(stream);
                println!("[IPC] Connected to simulator on {}", self.path);
            }
            let _ = listener.set_nonblocking(true);
        }
    }

    /// Accept pending incoming client connections without blocking the CPU
    pub fn poll_connection(&mut self) {
        if self.stream.is_none() {
            if let Some(ref listener) = self.listener {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = stream.set_nonblocking(true);
                    self.stream = Some(stream);
                }
            }
        }
    }

    /// Send raw bytes to connected external device
    pub fn send(&mut self, data: &[u8]) {
        self.poll_connection();
        if let Some(ref mut stream) = self.stream {
            if let Err(e) = stream.write_all(data) {
                if e.kind() == ErrorKind::BrokenPipe || e.kind() == ErrorKind::ConnectionReset {
                    self.stream = None; // Client disconnected
                }
            }
        }
    }

    /// Read available bytes into a buffer (returns number of bytes read)
    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        self.poll_connection();
        if let Some(ref mut stream) = self.stream {
            match stream.read(buf) {
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => 0,
                Err(_) => {
                    self.stream = None;
                    0
                }
            }
        } else {
            0
        }
    }

    /// Synchronous SPI Full-Duplex byte exchange with timeout
    pub fn spi_exchange(&mut self, cs_state: u8, mosi: u8) -> u8 {
        self.poll_connection();
        if let Some(ref mut stream) = self.stream {
            let packet = [0x02, cs_state, mosi];
            // Temporarily disable non-blocking for immediate reply
            let _ = stream.set_nonblocking(false);
            if stream.write_all(&packet).is_ok() {
                let mut miso = [0xFF];
                if stream.read_exact(&mut miso).is_ok() {
                    let _ = stream.set_nonblocking(true);
                    return miso[0];
                }
            }
            let _ = stream.set_nonblocking(true);
            self.stream = None;
        }
        0xFF // Default pull-up state when no external device is connected
    }
}

impl Drop for SocketBackend {
    fn drop(&mut self) {
        if Path::new(&self.path).exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}