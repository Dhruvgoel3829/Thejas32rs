import socket
import os
import time
import struct
import threading
import math

SPI_SOCK_PATH  = "/tmp/thejas_spi.sock"
GPIO_SOCK_PATH = "/tmp/thejas_gpio.sock"
GAZEBO_IMU_PORT = 9002
GAZEBO_MOTOR_PORT = 9003

physics_lock = threading.Lock()
latest_imu = {
    "ax": 0, "ay": 0, "az": 16384,
    "gx": 0, "gy": 0, "gz": 0
}

motor_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
motor_addr = ("127.0.0.1", GAZEBO_MOTOR_PORT)

# Synthetic IMU for testing without Gazebo
synthetic_tick = 0

def recv_exact(sock, n):
    buf = bytearray()
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)

def connect_socket(path, name):
    print(f"[{name}] Waiting for emulator socket at {path}...")
    while True:
        if os.path.exists(path):
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.connect(path)
                print(f"[{name}] Connected to emulator socket!")
                return s
            except (ConnectionRefusedError, FileNotFoundError):
                pass
        time.sleep(0.05)

# -------------------------------------------------------------
# 1. UDP IMU Receiver (from Gazebo plugin on port 9002)
# -------------------------------------------------------------
def udp_imu_receiver():
    global latest_imu, synthetic_tick
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", GAZEBO_IMU_PORT))
    sock.settimeout(1.0)
    print(f"[UDP-IMU] Listening on port {GAZEBO_IMU_PORT} for Gazebo IMU data...")
    last_gazebo_time = time.time()

    while True:
        try:
            data, addr = sock.recvfrom(1024)
            if len(data) == 56:  # 7 doubles = 56 bytes
                timestamp, ax, ay, az, gx, gy, gz = struct.unpack("7d", data)
                
                # Convert SI units to MPU-6000 register LSBs
                with physics_lock:
                    latest_imu["ax"] = int((ax / 9.80665) * 16384.0)
                    latest_imu["ay"] = int((ay / 9.80665) * 16384.0)
                    latest_imu["az"] = int((az / 9.80665) * 16384.0)
                    latest_imu["gx"] = int((gx * 57.2958) * 16.4)
                    latest_imu["gy"] = int((gy * 57.2958) * 16.4)
                    latest_imu["gz"] = int((gz * 57.2958) * 16.4)
                last_gazebo_time = time.time()
                # print(f"[UDP-IMU] REAL Gazebo: gx={latest_imu['gx']} gy={latest_imu['gy']} gz={latest_imu['gz']}")
        except socket.timeout:
            # Check if we haven't received Gazebo data for a while
            if time.time() - last_gazebo_time > 2.0:
                print("[UDP-IMU] WARNING: No Gazebo data for 2s, using synthetic!")
                with physics_lock:
                    phase = (synthetic_tick % 360) * (math.pi / 180.0)
                    synthetic_tick += 1
                    latest_imu["ax"] = int(math.sin(phase) * 800)
                    latest_imu["ay"] = 0
                    latest_imu["az"] = 16384 - int(abs(math.sin(phase)) * 100)
                    latest_imu["gx"] = int(math.sin(phase) * 410)
                    latest_imu["gy"] = int(math.cos(phase) * 205)
                    latest_imu["gz"] = 0
            else:
                # Just keep last known good values
                pass
        except Exception as e:
            print(f"[UDP-IMU] Error: {e}")

# -------------------------------------------------------------
# 2. SPI Socket Server (Virtual MPU-6000 Register Machine)
# -------------------------------------------------------------
def spi_imu_server():
    print("[SPI-SERVER] Starting...")
    client = connect_socket(SPI_SOCK_PATH, "IMU")
    print("[SPI-SERVER] Connected, entering main loop")
    selected = False
    current_reg = 0
    is_read = False
    byte_idx = 0

    try:
        while True:
            hdr = recv_exact(client, 1)
            if not hdr: break
            pkt_type = hdr[0]
            print(f"[SPI-SERVER] pkt_type=0x{pkt_type:02X}")

            if pkt_type == 0x01: # CS Line Transition
                cs_payload = recv_exact(client, 1)
                if not cs_payload: break
                selected = (cs_payload[0] & 0x01) == 0
                if not selected:
                    byte_idx = 0

            elif pkt_type == 0x02: # Full-Duplex Byte Transfer
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
                    if reg == 0x75:
                        miso = 0x68 # WHO_AM_I
                    elif 0x3B <= reg <= 0x48:
                        with physics_lock:
                            ax, ay, az = latest_imu["ax"], latest_imu["ay"], latest_imu["az"]
                            gx, gy, gz = latest_imu["gx"], latest_imu["gy"], latest_imu["gz"]

                        burst_table = {
                            0x3B: (ax >> 8) & 0xFF, 0x3C: ax & 0xFF,
                            0x3D: (ay >> 8) & 0xFF, 0x3E: ay & 0xFF,
                            0x3F: (az >> 8) & 0xFF, 0x40: az & 0xFF,
                            0x41: 0x09,             0x42: 0xC4, #~25C
                            0x43: (gx >> 8) & 0xFF, 0x44: gx & 0xFF,
                            0x45: (gy >> 8) & 0xFF, 0x46: gy & 0xFF,
                            0x47: (gz >> 8) & 0xFF, 0x48: gz & 0xFF,
                        }
                        miso = burst_table.get(reg, 0x00)
                    else:
                        miso = 0x00
                    client.sendall(bytes([miso]))
                else:
                    client.sendall(b'\x00')
    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

# -------------------------------------------------------------
# 3. Motor Actuation (Forward to Gazebo plugin on UDP 9003)
# -------------------------------------------------------------
def motor_worker():
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

            motor_floats = struct.pack("4f", m1/255.0, m2/255.0, m3/255.0, m4/255.0)
            motor_sock.sendto(motor_floats, motor_addr)

    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()
        motor_sock.close()

def main():
    t_udp = threading.Thread(target=udp_imu_receiver, daemon=True)
    t_spi = threading.Thread(target=spi_imu_server, daemon=True)
    t_motor = threading.Thread(target=motor_worker, daemon=True)

    t_udp.start()
    t_spi.start()
    t_motor.start()

    try:
        while True:
            time.sleep(0.5)
    except KeyboardInterrupt:
        print("\n[SITL BRIDGE] Shutting down bridge cleanly...")

if __name__ == "__main__":
    main()