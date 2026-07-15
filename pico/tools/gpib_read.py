#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pyserial>=3.5"]
# ///

import argparse
import os
import time
from pathlib import Path

import serial


OUTPUT_FILE = "gpib-read.bin"
RESPONSE_TIMEOUT = 5
SECTOR_SIZE = 512
READ_ATTEMPTS = 3


def parse_device(value: str) -> int:
    device = int(value)
    if not 0 <= device <= 30:
        raise argparse.ArgumentTypeError("GPIB device must be in range 0..30")
    return device


def parse_non_negative(value: str) -> int:
    number = int(value)
    if number < 0:
        raise argparse.ArgumentTypeError("value must not be negative")
    return number


def read_response(port: serial.Serial) -> bytes:
    deadline = time.monotonic() + RESPONSE_TIMEOUT

    while time.monotonic() < deadline:
        line = port.readline().decode(errors="replace").strip()
        if not line:
            continue
        if line == "ERROR":
            raise RuntimeError("adapter returned ERROR")
        try:
            response = bytes.fromhex(line)
        except ValueError:
            continue
        return response

    raise TimeoutError("sector response timeout")


def read_sector(port: serial.Serial, device: int, sector: int) -> tuple[bytes, int | None]:
    error = None

    for _ in range(READ_ATTEMPTS):
        port.write(f"AT+READ={device},{sector}\r\n".encode())
        port.flush()

        response = read_response(port)
        if len(response) == SECTOR_SIZE:
            return response, None
        if len(response) != 7:
            raise RuntimeError(f"sector {sector}: expected 7 or {SECTOR_SIZE} bytes, got {len(response)}")
        error = int.from_bytes(response[:2], "little")

    return b"\xFF" * SECTOR_SIZE, error


def print_progress(sector: int, total_sectors: int) -> None:
    width = 40
    filled = width if total_sectors == 0 else width * sector // total_sectors
    print(f"\r[{'#' * filled}{'.' * (width - filled)}] {sector}/{total_sectors}", end="", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tty", required=True)
    parser.add_argument("--gpib", required=True, type=parse_device)
    parser.add_argument("--sectors", required=True, type=parse_non_negative)
    parser.add_argument("--start-sector", default=0, type=parse_non_negative)
    parser.add_argument("--file", default=Path(OUTPUT_FILE), type=Path)
    args = parser.parse_args()
    if args.start_sector > args.sectors:
        parser.error("--start-sector must not exceed --sectors")

    mode = "r+b" if args.file.exists() else "w+b"
    with (
        serial.Serial(args.tty, 115200, timeout=0.2) as port,
        open(args.file, mode, buffering=0) as output,
    ):
        output.seek(args.start_sector * SECTOR_SIZE)
        print_progress(args.start_sector, args.sectors)
        for sector in range(args.start_sector, args.sectors):
            response, error = read_sector(port, args.gpib, sector)
            if error is not None:
                print(f"\rSector {sector}: error 0x{error:04X} after {READ_ATTEMPTS} attempts")

            output.write(response)
            os.fsync(output.fileno())
            print_progress(sector + 1, args.sectors)
        print()


if __name__ == "__main__":
    main()
