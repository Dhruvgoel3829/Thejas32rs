import socket
import os
import time
import struct

SOCKET_PATH = "/tmp/thejas_gpio.sock"

def connect_to_emulator(path):
    print(f"[MOTOR PROXY] Waiting for emulator socket at {path}...")
    while True:
        if os.path.exists(path):
            try:
                client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                client.connect(path)
                print(f"[MOTOR PROXY] Connected to {path}!\n")
                return client
            except (ConnectionRefusedError, FileNotFoundError):
                pass
        time.sleep(0.05)

def main():
    client = connect_to_emulator(SOCKET_PATH)

    print("=" * 60)
    print("  M1 (Rear Right) | M2 (Front Right) | M3 (Rear Left) | M4 (Front Left)")
    print("=" * 60)

    try:
        while True:
            # Read exactly 4 bytes (u32 little-endian)
            payload = bytearray()
            while len(payload) < 4:
                chunk = client.recv(4 - len(payload))
                if not chunk:
                    return
                payload.extend(chunk)

            m1, m2, m3, m4 = struct.unpack("4B", payload)

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