import socket
import os
import time
import math
import struct
import threading

SPI_SOCK_PATH = "/tmp/thejas_spi.sock"
GPIO_SOCK_PATH = "/tmp/thejas_gpio.sock"

def recv_exact(sock, n):
    buf = bytearray()
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)

def connect_socket(path, name):
    print(f"[{name}] Waiting for socket at {path}...")
    while True:
        if os.path.exists(path):
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.connect(path)
                print(f"[{name}] Connected!")
                return s
            except (ConnectionRefusedError, FileNotFoundError):
                pass
        time.sleep(0.05)

# -------------------------------------------------------------
# Motor Receiver Thread (GPIO)
# -------------------------------------------------------------
def motor_thread():
    client = connect_socket(GPIO_SOCK_PATH, "MOTOR")
    print("=" * 60)
    print("  M1 (RR)      | M2 (FR)      | M3 (RL)      | M4 (FL)")
    print("=" * 60)
    try:
        while True:
            payload = recv_exact(client, 4)
            if not payload:
                break
            m1, m2, m3, m4 = struct.unpack("4B", payload)
            print(f"  {m1:3d} ({(m1/255)*100:4.1f}%) | {m2:3d} ({(m2/255)*100:4.1f}%) | {m3:3d} ({(m3/255)*100:4.1f}%) | {m4:3d} ({(m4/255)*100:4.1f}%)")
    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

# -------------------------------------------------------------
# IMU Server Thread (SPI)
# -------------------------------------------------------------
def imu_thread():
    client = connect_socket(SPI_SOCK_PATH, "IMU")
    tick = 0
    selected = False
    current_reg = 0
    is_read = False
    byte_idx = 0

    try:
        while True:
            hdr = recv_exact(client, 1)
            if not hdr: break
            pkt_type = hdr[0]

            if pkt_type == 0x01: # CS
                cs_payload = recv_exact(client, 1)
                if not cs_payload: break
                selected = (cs_payload[0] & 0x01) == 0
                if not selected:
                    byte_idx = 0

            elif pkt_type == 0x02: # SPI Transfer
                payload = recv_exact(client, 2)
                if not payload: break
                cs_state, mosi = payload[0], payload[1]
                selected = (cs_state & 0x01) == 0

                if not selected:
                    client.sendall(b'\xFF')
                    continue

                if byte_idx == 0:
                    is_read = (mosi & 0x80) != 0
                    current_reg = mosi & 0x7F
                    byte_idx += 1
                    client.sendall(b'\x00')
                    continue

                reg = current_reg
                current_reg = (current_reg + 1) & 0x7F
                byte_idx += 1

                if is_read:
                    if reg == 0x75: miso = 0x68
                    elif reg == 0x3F: miso = 0x40 # Accel Z = 16384 (1g)
                    elif reg == 0x40: miso = 0x00
                    elif reg == 0x43: # Gyro X high (Dynamic disturbance)
                        tick += 1
                        val = int(math.sin(tick * 0.2) * 320) # Up to ~20 dps disturbance
                        miso = (val >> 8) & 0xFF
                    elif reg == 0x44: # Gyro X low
                        val = int(math.sin(tick * 0.2) * 320)
                        miso = val & 0xFF
                    else:
                        miso = 0x00
                    client.sendall(bytes([miso]))
                else:
                    client.sendall(b'\x00')
    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

def main():
    t_motor = threading.Thread(target=motor_thread, daemon=True)
    t_imu = threading.Thread(target=imu_thread, daemon=True)

    t_motor.start()
    t_imu.start()

    t_motor.join()
    t_imu.join()

if __name__ == "__main__":
    main()