#!/usr/bin/env python3
# tools/external_imu_proxy.py
import socket
import os
import time
import struct
import math

SOCKET_PATH = "/tmp/thejas_spi.sock"

class ExternalMpu6000:
    def __init__(self):
        self.selected = False
        self.current_reg = 0
        self.is_read = False
        self.byte_idx = 0
        self.tick = 0

    def handle_cs(self, cs_byte):
        self.selected = (cs_byte & 0x01) == 0
        if not self.selected:
            self.byte_idx = 0

    def handle_byte(self, mosi):
        if not self.selected:
            return 0xFF

        if self.byte_idx == 0:
            self.is_read = (mosi & 0x80) != 0
            self.current_reg = mosi & 0x7F
            self.byte_idx += 1
            return 0x00

        self.tick += 1
        reg = self.current_reg
        self.current_reg = (self.current_reg + 1) & 0x7F
        self.byte_idx += 1

        if self.is_read:
            # WHO_AM_I
            if reg == 0x75: return 0x68
            # Accel Z (16384 = 1.0g baseline)
            if reg == 0x3F: return 0x40
            if reg == 0x40: return 0x00
            # Gyro X (Simulated live sine wave roll rate)
            if reg == 0x43:
                val = int(math.sin(self.tick * 0.1) * 300)
                return (val >> 8) & 0xFF
            if reg == 0x44:
                val = int(math.sin(self.tick * 0.1) * 300)
                return val & 0xFF
            return 0x00
        else:
            return 0x00

def main():
    print(f"[PROXY] Waiting for emulator to open {SOCKET_PATH}...")
    while not os.path.exists(SOCKET_PATH):
        time.sleep(0.05)

    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.connect(SOCKET_PATH)
    print(f"[PROXY] Connected to Thejas32rs SPI socket.")

    imu = ExternalMpu6000()

    try:
        while True:
            pkt = client.recv(3)
            if not pkt:
                break

            pkt_type = pkt[0]
            if pkt_type == 0x01: # CS Line Change
                imu.handle_cs(pkt[1])
            elif pkt_type == 0x02: # SPI Byte Exchange
                cs_state = pkt[1]
                mosi = pkt[2]
                imu.handle_cs(cs_state)
                miso = imu.handle_byte(mosi)
                client.sendall(bytes([miso]))
    except KeyboardInterrupt:
        pass
    finally:
        client.close()

if __name__ == "__main__":
    main()