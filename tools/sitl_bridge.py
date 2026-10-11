import socket
import os
import time
import struct
import threading
import math

SPI_SOCK_PATH  = "/tmp/thejas_spi.sock"
GPIO_SOCK_PATH = "/tmp/thejas_gpio.sock"
I2C_SOCK_PATH  = "/tmp/thejas_i2c.sock"
UART_SOCK_PATH = "/tmp/thejas_uart.sock"
GAZEBO_IMU_PORT = 9002
GAZEBO_MOTOR_PORT = 9003

physics_lock = threading.Lock()
latest_imu = {
    "ax": 0, "ay": 0, "az": 16384,
    "gx": 0, "gy": 0, "gz": 0
}

# I2C sensor data (from Gazebo)
latest_baro = {"pressure": 101325.0, "temperature": 25.0, "altitude": 0.0}
latest_compass = {"mx": 0.0, "my": 0.0, "mz": 0.0}
latest_gps = {"lat": 0.0, "lon": 0.0, "alt": 0.0, "vel_n": 0.0, "vel_e": 0.0, "vel_d": 0.0, "fix": 3, "sats": 10}

motor_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
motor_addr = ("127.0.0.1", GAZEBO_MOTOR_PORT)

# Synthetic IMU for testing without Gazebo
synthetic_tick = 0

# UART buffer for NMEA/telemetry output
uart_output_buffer = []
uart_buffer_lock = threading.Lock()

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

# -------------------------------------------------------------
# 4. I2C Socket Server (Virtual I2C Sensors: Baro, Compass, GPS)
# -------------------------------------------------------------
def i2c_sensor_server():
    print("[I2C-SERVER] Starting...")
    client = connect_socket(I2C_SOCK_PATH, "I2C")
    print("[I2C-SERVER] Connected, entering main loop")

    # Extended Protocol from emulator:
    # Write: [0x00, addr, reg?, data...] - variable length
    #        If reg is known from previous write, can omit (command-only)
    # Read:  [0x01, addr, reg?, num_bytes] -> response: num_bytes of data
    #        reg optional if previously set
    current_addr = 0
    current_reg = 0
    addr_latch = False  # Tracks if reg was set by previous write

    try:
        while True:
            hdr = recv_exact(client, 1)
            if not hdr: break
            op_type = hdr[0]

            if op_type == 0x00:  # Write
                # Read variable length payload: minimum 2 bytes (addr, data), up to 255
                # Format: [0x00, addr, reg?, data...]
                # First read addr + at least 1 byte
                payload = recv_exact(client, 2)
                if not payload: break
                addr = payload[0]
                current_addr = addr
                
                # Check if there's more data
                remaining = client.recv(255, socket.MSG_PEEK)
                if remaining:
                    more = recv_exact(client, len(remaining))
                    payload = payload + more
                else:
                    more = b''

                if len(payload) >= 3:
                    # Format: [addr, reg, data...] - reg provided
                    reg = payload[1]
                    data = payload[2:]
                    current_reg = reg
                    addr_latch = True
                elif len(payload) == 2:
                    # Format: [addr, data] - command-only write (no register)
                    data = payload[1:]
                    addr_latch = False
                else:
                    continue

                # Handle writes to external I2C sensors
                if addr == 0x77:  # MS5611 Barometer
                    # MS5611 commands: 0x1E (reset), 0x40-0x46 (D1/D2 conversions), 0xA0-0xAE (PROM reads)
                    # Command-only writes (no register): 0x1E, 0x40-0x46
                    if not addr_latch and len(data) >= 1:
                        cmd = data[0]
                        if cmd == 0x1E:
                            # Reset command
                            pass
                        elif 0x40 <= cmd <= 0x46:
                            # D1/D2 conversion commands
                            pass
                elif addr == 0x1E:  # HMC5883L Compass
                    # HMC5883L registers - write with register
                    pass
                elif addr == 0x42:  # GPS (common I2C address)
                    # GPS registers
                    pass

            elif op_type == 0x01:  # Read
                # Format: [0x01, addr, reg?, num_bytes]
                payload = recv_exact(client, 2)
                if not payload: break
                addr = payload[0]
                current_addr = addr
                
                # Check for optional reg + num_bytes
                remaining = client.recv(255, socket.MSG_PEEK)
                if remaining:
                    more = recv_exact(client, len(remaining))
                    payload = payload + more

                if len(payload) >= 4:
                    # Format: [addr, reg, num_bytes]
                    reg = payload[1]
                    num_bytes = payload[2]
                    current_reg = reg
                    addr_latch = True
                elif len(payload) == 3:
                    # Format: [addr, num_bytes] - use latched reg
                    num_bytes = payload[1]
                    reg = current_reg
                    if not addr_latch:
                        num_bytes = 1  # Default to 1 byte
                else:
                    num_bytes = 1

                # Handle reads from external I2C sensors
                response = bytearray(num_bytes)

                if addr == 0x77:  # MS5611 Barometer
                    with physics_lock:
                        # MS5611: 3-byte ADC result (pressure or temp)
                        if num_bytes >= 3:
                            pressure = int(latest_baro["pressure"] * 100)  # Pa to cPa
                            response[0] = (pressure >> 16) & 0xFF
                            response[1] = (pressure >> 8) & 0xFF
                            response[2] = pressure & 0xFF
                        elif num_bytes >= 1:
                            # Temperature (simplified)
                            temp = int(latest_baro["temperature"] * 100)
                            response[0] = (temp >> 8) & 0xFF
                            if num_bytes >= 2:
                                response[1] = temp & 0xFF

                elif addr == 0x1E:  # HMC5883L Compass
                    with physics_lock:
                        # HMC5883L: 6-byte burst read (X MSB, X LSB, Z MSB, Z LSB, Y MSB, Y LSB)
                        mx = int(latest_compass["mx"] * 1000)
                        my = int(latest_compass["my"] * 1000)
                        mz = int(latest_compass["mz"] * 1000)
                        if num_bytes >= 6:
                            response[0] = (mx >> 8) & 0xFF  # X MSB
                            response[1] = mx & 0xFF         # X LSB
                            response[2] = (mz >> 8) & 0xFF  # Z MSB
                            response[3] = mz & 0xFF         # Z LSB
                            response[4] = (my >> 8) & 0xFF  # Y MSB
                            response[5] = my & 0xFF         # Y LSB
                        elif num_bytes >= 1:
                            # Single register reads (backward compat)
                            if reg == 0x03:
                                response[0] = (mx >> 8) & 0xFF
                            elif reg == 0x04:
                                response[0] = mx & 0xFF
                            elif reg == 0x05:
                                response[0] = (mz >> 8) & 0xFF
                            elif reg == 0x06:
                                response[0] = mz & 0xFF
                            elif reg == 0x07:
                                response[0] = (my >> 8) & 0xFF
                            elif reg == 0x08:
                                response[0] = my & 0xFF
                            elif reg == 0x0A:  # Status
                                response[0] = 0x01  # Data ready
                            elif reg == 0x0B:  # Config A
                                response[0] = 0x70
                            elif reg == 0x0C:  # Config B
                                response[0] = 0xA0
                            elif reg == 0x0D:  # Mode
                                response[0] = 0x00

                elif addr == 0x42:  # GPS (common I2C)
                    with physics_lock:
                        lat = int(latest_gps["lat"] * 1e7)
                        lon = int(latest_gps["lon"] * 1e7)
                        alt = int(latest_gps["alt"] * 1000)
                        if num_bytes >= 4:
                            response[0] = (lat >> 24) & 0xFF
                            response[1] = (lat >> 16) & 0xFF
                            response[2] = (lat >> 8) & 0xFF
                            response[3] = lat & 0xFF
                            if num_bytes >= 8:
                                response[4] = (lon >> 24) & 0xFF
                                response[5] = (lon >> 16) & 0xFF
                                response[6] = (lon >> 8) & 0xFF
                                response[7] = lon & 0xFF

                client.sendall(response)

    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

# -------------------------------------------------------------
# 5. UART Socket Server (Virtual UART: GPS NMEA, Telemetry)
# -------------------------------------------------------------
def uart_server():
    print("[UART-SERVER] Starting...")
    client = connect_socket(UART_SOCK_PATH, "UART")
    print("[UART-SERVER] Connected, entering main loop")

    try:
        while True:
            # Read byte from emulator (written to THR)
            payload = recv_exact(client, 1)
            if not payload:
                break
            byte = payload[0]

            # Echo back for local terminal (emulator writes to THR, we can log it)
            # For GPS NMEA output, we can inject NMEA sentences here
            # The emulator reads from RHR, so we need to provide data when read

            # Log the byte sent by emulator (for telemetry debugging)
            if 32 <= byte <= 126:  # Printable ASCII
                with uart_buffer_lock:
                    uart_output_buffer.append(chr(byte))
                    if len(uart_output_buffer) > 1000:
                        uart_output_buffer = uart_output_buffer[-1000:]

            # Acknowledge receipt (for flow control)
            client.sendall(b'\x00')

    except (BrokenPipeError, ConnectionResetError):
        pass
    finally:
        client.close()

def nmea_sender():
    """Periodically send GPS NMEA sentences to UART RX buffer"""
    # This would be called from the I2C server or separately
    # For now, we handle GPS via I2C
    pass

def main():
    t_udp = threading.Thread(target=udp_imu_receiver, daemon=True)
    t_spi = threading.Thread(target=spi_imu_server, daemon=True)
    t_motor = threading.Thread(target=motor_worker, daemon=True)
    t_i2c = threading.Thread(target=i2c_sensor_server, daemon=True)
    t_uart = threading.Thread(target=uart_server, daemon=True)

    t_udp.start()
    t_spi.start()
    t_motor.start()
    t_i2c.start()
    t_uart.start()

    try:
        while True:
            time.sleep(0.5)
    except KeyboardInterrupt:
        print("\n[SITL BRIDGE] Shutting down bridge cleanly...")

if __name__ == "__main__":
    main()