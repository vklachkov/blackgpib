#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pyserial>=3.5"]
# ///

import argparse
import struct
import time

import serial


STATUS_SIZE = 52
VERSION_TIMEOUT = 1
CONNECT_TIMEOUT = 10


def parse_device(value: str) -> int:
    device = int(value)
    if not 0 <= device <= 30:
        raise argparse.ArgumentTypeError("GPIB device must be in range 0..30")
    return device


def send(port: serial.Serial, command: str) -> None:
    port.write(f"{command}\r\n".encode())
    port.flush()


def read_version(port: serial.Serial) -> str | None:
    port.reset_input_buffer()
    send(port, "AT+VERSION")
    deadline = time.monotonic() + VERSION_TIMEOUT

    while time.monotonic() < deadline:
        line = port.readline().decode(errors="replace").strip()
        if line.startswith("BLACKGPIB-"):
            return line
    return None


def open_adapter(tty: str) -> tuple[serial.Serial, str]:
    port = serial.Serial(tty, 115200, timeout=0.2)
    version = read_version(port)
    if version is not None:
        return port, version

    send(port, "AT+CONNECT")
    port.close()

    deadline = time.monotonic() + CONNECT_TIMEOUT
    while time.monotonic() < deadline:
        try:
            port = serial.Serial(tty, 115200, timeout=0.2)
        except serial.SerialException:
            time.sleep(0.2)
            continue

        version = read_version(port)
        if version is not None:
            return port, version
        port.close()
        time.sleep(0.2)

    raise TimeoutError("adapter did not reconnect")


def read_status(port: serial.Serial) -> bytes:
    deadline = time.monotonic() + 5

    while time.monotonic() < deadline:
        line = port.readline().decode(errors="replace").strip()
        if not line:
            continue
        if line == "ERROR":
            raise RuntimeError("adapter returned ERROR")
        try:
            data = bytes.fromhex(line)
        except ValueError:
            continue
        if len(data) != STATUS_SIZE:
            raise RuntimeError(f"expected {STATUS_SIZE} status bytes, got {len(data)}")
        return data

    raise TimeoutError("status response timeout")


def print_status(data: bytes) -> None:
    (
        sector_size,
        logical_sector_size,
        sector_count,
        drive_status,
        bitmap_block_id,
        superblock_id,
        min_dir_pages,
        flush,
    ) = struct.unpack_from("<HHHBHHHB", data)
    name = data[14:46].decode("ascii", errors="replace").rstrip("\0 ")
    bytes_per_sector, sectors_per_track, tracks_per_cylinder = struct.unpack_from("<HHH", data, 46)
    drive_state = {0: "not ready", 1: "ready", 3: "error"}.get(drive_status, "unknown")

    print(f"Sector size: {sector_size}")
    print(f"Logical sector size: {logical_sector_size}")
    print(f"Sector count: {sector_count}")
    print(f"Drive status: {drive_status} ({drive_state})")
    print(f"Bitmap block ID: {bitmap_block_id}")
    print(f"Superblock ID: {superblock_id}")
    print(f"Min directory pages: {min_dir_pages}")
    print(f"Flush: {flush}")
    print(f"Device name: {name}")
    print(f"Bytes per sector: {bytes_per_sector}")
    print(f"Sectors per track: {sectors_per_track}")
    print(f"Tracks per cylinder: {tracks_per_cylinder}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("tty")
    parser.add_argument("device", type=parse_device)
    args = parser.parse_args()

    port, version = open_adapter(args.tty)
    try:
        print(version)
        if input("Reset GPIB device? [y/N] ").strip().lower() in {"y", "yes"}:
            send(port, "AT+GPIB_RESET")
            time.sleep(15)

        port.reset_input_buffer()
        send(port, f"AT+STATUS={args.device}")
        print_status(read_status(port))
    finally:
        port.close()


if __name__ == "__main__":
    main()
