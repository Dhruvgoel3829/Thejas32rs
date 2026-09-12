import socket
import os
import time
import math
import sys

SOCKET_PATH = "/tmp/thejas_spi.sock"

def recv_exact(sock, n):
    """Guarantees exactly n bytes are read from the stream socket."""
    buf = bytearray()
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)

class ExternalMpu6000:
    def __init__(self):
        self.selected = False
        self.current_reg = 0
        self.is_read = False
        self.byte_idx = 0
        self.tick = 0
        self.registers = {
            0x75: 0x68, # WHO_AM_I
            0x6B: 0x40, # PWR_MGMT_1
            0x1B: 0x00, # GYRO_CONFIG
            0x1C: 0x00, # ACCEL_CONFIG
        }

    def handle_cs(self, cs_byte):
        was_selected = self.selected
        self.selected = (cs_byte & 0x01) == 0
        
        # Reset SPI transaction state machine when CS goes HIGH (deselected)
        if not self.selected:
            self.byte_idx = 0
            self.current_reg = 0

    def handle_byte(self, mosi):
        if not self.selected:
            return 0xFF

        # First byte in an active CS assertion is the Command Byte [R/W | 7-bit Addr]
        if self.byte_idx == 0:
            self.is_read = (mosi & 0x80) != 0
            self.current_reg = mosi & 0x7F
            self.byte_idx += 1
            return 0x00

        reg = self.current_reg
        self.byte_idx += 1
        # MPU-6000 auto-increments register address during multi-byte transfers
        self.current_reg = (self.current_reg + 1) & 0x7F

        if self.is_read:
            return self.read_reg(reg)
        else:
            self.registers[reg] = mosi
            return 0x00

    def read_reg(self, reg):
        self.tick += 1
        phase = (self.tick % 360) * (math.pi / 180.0)

        # Synthetic quadcopter dynamics
        ax = int(math.sin(phase) * 800)
        ay = 0
        az = 16384 - int(abs(math.sin(phase)) * 100)

        gx = int(math.sin(phase) * 410)
        gy = int(math.cos(phase) * 205)
        gz = 0

        temp = 2500

        burst_map = {
            0x3B: (ax >> 8) & 0xFF,
            0x3C: ax & 0xFF,
            0x3D: (ay >> 8) & 0xFF,
            0x3E: ay & 0xFF,
            0x3F: (az >> 8) & 0xFF,
            0x40: az & 0xFF,
            0x41: (temp >> 8) & 0xFF,
            0x42: temp & 0xFF,
            0x43: (gx >> 8) & 0xFF,
            0x44: gx & 0xFF,
            0x45: (gy >> 8) & 0xFF,
            0x46: gy & 0xFF,
            0x47: (gz >> 8) & 0xFF,
            0x48: gz & 0xFF,
        }

        return burst_map.get(reg, self.registers.get(reg, 0x00))

def main():
    print(f"[PROXY] Waiting for emulator socket at {SOCKET_PATH}...")
    while not os.path.exists(SOCKET_PATH):
        time.sleep(0.05)

    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.connect(SOCKET_PATH)
    print(f"[PROXY] Connected to Thejas32rs SPI bus!")

    imu = ExternalMpu6000()

    try:
        while True:
            # 1. Read packet type header (1 byte)
            hdr = recv_exact(client, 1)
            if not hdr:
                break

            pkt_type = hdr[0]

            if pkt_type == 0x01:
                # CS Line event: exactly 1 payload byte
                payload = recv_exact(client, 1)
                if not payload: break
                imu.handle_cs(payload[0])

            elif pkt_type == 0x02:
                # SPI transfer event: exactly 2 payload bytes [cs_state, mosi]
                payload = recv_exact(client, 2)
                if not payload: break
                cs_state, mosi = payload[0], payload[1]
                imu.handle_cs(cs_state)
                miso = imu.handle_byte(mosi)
                # Send back 1 byte MISO
                client.sendall(bytes([miso]))

    except (KeyboardInterrupt, BrokenPipeError):
        pass
    finally:
        client.close()
        print("\n[PROXY] Disconnected.")

if __name__ == "__main__":
    main()