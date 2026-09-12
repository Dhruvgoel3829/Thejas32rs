import socket
import os
import time
import struct

SOCKET_PATH = "/tmp/thejas_gpio.sock"

def main():
    print(f"[MOTOR PROXY] Waiting for emulator socket at {SOCKET_PATH}...")
    while not os.path.exists(SOCKET_PATH):
        time.sleep(0.05)

    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.connect(SOCKET_PATH)
    print(f"[MOTOR PROXY] Connected to /tmp/thejas_gpio.sock!\n")
    print("=" * 60)
    print("  M1 (Rear Right) | M2 (Front Right) | M3 (Rear Left) | M4 (Front Left)")
    print("=" * 60)

    try:
        while True:
            # Each GPIO_DATA write emits a 4-byte little-endian u32
            payload = client.recv(4)
            if not payload or len(payload) < 4:
                break

            m1, m2, m3, m4 = struct.unpack("4B", payload)

            # Convert 0-255 range to percentages
            pct_m1 = (m1 / 255.0) * 100
            pct_m2 = (m2 / 255.0) * 100
            pct_m3 = (m3 / 255.0) * 100
            pct_m4 = (m4 / 255.0) * 100

            print(f"  {m1:3d} ({pct_m1:5.1f}%)    |  {m2:3d} ({pct_m2:5.1f}%)     |  {m3:3d} ({pct_m3:5.1f}%)    |  {m4:3d} ({pct_m4:5.1f}%)")

    except (KeyboardInterrupt, BrokenPipeError):
        pass
    finally:
        client.close()
        print("\n[MOTOR PROXY] Disconnected.")

if __name__ == "__main__":
    main()