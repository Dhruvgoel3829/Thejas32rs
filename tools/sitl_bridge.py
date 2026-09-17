import socket
import os
import time
import struct
import threading
import subprocess
import re

SPI_SOCK_PATH  = "/tmp/thejas_spi.sock"
GPIO_SOCK_PATH = "/tmp/thejas_gpio.sock"

# Topic name published by Gazebo native IMU sensor
GAZEBO_IMU_TOPIC = "~/iris/base_link/imu_sensor/imu"

physics_lock = threading.Lock()
latest_imu = {
    "ax": 0, "ay": 0, "az": 16384,
    "gx": 0, "gy": 0, "gz": 0
}

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
# 1. Gazebo Topic Ingestion Worker (Zero C++ Plugins)
# -------------------------------------------------------------
def gazebo_cli_imu_worker():
    global latest_imu
    print(f"[GAZEBO-CLI] Subscribing to {GAZEBO_IMU_TOPIC} via gz topic...")

    # Wait for Gazebo to actually start and advertise topics
    while True:
        try:
            topics = subprocess.check_output(["gz", "topic", "-l"], stderr=subprocess.DEVNULL).decode()
            if "imu" in topics:
                # Find matching imu topic dynamically if path differs
                for line in topics.strip().splitlines():
                    if "imu_sensor/imu" in line:
                        target_topic = line.strip()
                        break
                else:
                    target_topic = GAZEBO_IMU_TOPIC
                break
        except Exception:
            pass
        time.sleep(0.5)

    print(f"[GAZEBO-CLI] Streaming active on: {target_topic}")

    proc = subprocess.Popen(
        ["gz", "topic", "-e", target_topic],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        bufsize=1
    )

    rex_accel = re.compile(r"linear_acceleration\s*\{\s*x:\s*([-\d\.e\+]+)\s*y:\s*([-\d\.e\+]+)\s*z:\s*([-\d\.e\+]+)")
    rex_gyro  = re.compile(r"angular_velocity\s*\{\s*x:\s*([-\d\.e\+]+)\s*y:\s*([-\d\.e\+]+)\s*z:\s*([-\d\.e\+]+)")

    current_chunk = []

    for line in iter(proc.stdout.readline, ''):
        current_chunk.append(line)
        if "orientation" in line and len(current_chunk) > 10:
            block = "".join(current_chunk)
            current_chunk = []

            m_a = rex_accel.search(block)
            m_g = rex_gyro.search(block)

            if m_a and m_g:
                try:
                    ax_m, ay_m, az_m = float(m_a.group(1)), float(m_a.group(2)), float(m_a.group(3))
                    gx_r, gy_r, gz_r = float(m_g.group(1)), float(m_g.group(2)), float(m_g.group(3))

                    # Convert SI units to MPU-6000 register LSBs:
                    # 1g = 9.80665 m/s^2 = 16384 LSB (at +/- 2g range)
                    # 1 rad/s = 57.2958 deg/s. At +/- 2000 dps: 16.4 LSB/dps
                    with physics_lock:
                        latest_imu["ax"] = int((ax_m / 9.80665) * 16384.0)
                        latest_imu["ay"] = int((ay_m / 9.80665) * 16384.0)
                        latest_imu["az"] = int((az_m / 9.80665) * 16384.0)

                        latest_imu["gx"] = int((gx_r * 57.2958) * 16.4)
                        latest_imu["gy"] = int((gy_r * 57.2958) * 16.4)
                        latest_imu["gz"] = int((gz_r * 57.2958) * 16.4)
                except ValueError:
                    pass

# -------------------------------------------------------------
# 2. SPI Socket Server (Virtual MPU-6000 Register Machine)
# -------------------------------------------------------------
def spi_imu_server():
    client = connect_socket(SPI_SOCK_PATH, "IMU")
    selected = False
    current_reg = 0
    is_read = False
    byte_idx = 0

    try:
        while True:
            hdr = recv_exact(client, 1)
            if not hdr: break
            pkt_type = hdr[0]

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
# 3. Motor Actuation (Applies Forces or Prints PWM Outputs)
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
    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

def main():
    t_gz = threading.Thread(target=gazebo_cli_imu_worker, daemon=True)
    t_spi = threading.Thread(target=spi_imu_server, daemon=True)
    t_motor = threading.Thread(target=motor_worker, daemon=True)

    t_gz.start()
    t_spi.start()
    t_motor.start()

    try:
        while True:
            time.sleep(0.5)
    except KeyboardInterrupt:
        print("\n[SITL BRIDGE] Shutting down bridge cleanly...")

if __name__ == "__main__":
    main()