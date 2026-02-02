import csv
import os
import sys

def parse_bits(bits):
    res = 0
    for bit in bits[::-1]:
        res <<= 1
        val = int(bit) ^ 1
        res += val
    return res

def main():
    if len(sys.argv) < 2:
        print("Usage: {} <CSV file> [start time]".format(sys.argv[0]))
        return -1

    csv_file = sys.argv[1]
    with open(csv_file) as f:
        csv_lines = f.readlines()
    
    start_seconds = 0
    if len(sys.argv) == 3:
        start_seconds = int(sys.argv[2])

    header = csv_lines[0]
    header_items = header.strip("\n").split(',')

    time_idx = header_items.index("Time [s]")
    dav_idx = header_items.index("DAV")
    atn_idx = header_items.index("ATN")
    eoi_idx = header_items.index("EOI")

    ch0_idx = header_items.index("DIO1")

    should_print = True
    is_error = False
    dio_lines = []
    atn_val = 0
    eoi_val = 0
    ts_seconds_old = 0
    ts_milliseconds_old = 0 
    dt_threshold = 100  # ns
    values_older, values_old = None, None
    for idx, line in enumerate(csv_lines[1:]):
        values = line.strip("\n").split(',')

        ts_seconds, ts_milliseconds = [int (x) for x in values[0].split(".")]

        if ts_seconds_old != 0 and ts_milliseconds_old != 0 and ts_seconds >= start_seconds and idx > 1:
            dt = abs(ts_milliseconds - ts_milliseconds_old)
            if dt < dt_threshold:
                diff_channels = []
                for i, val in enumerate(values):
                    if values_older[i] == val and values_old[i] != val:
                        diff_channels.append(header_items[i])
                    
                if len(diff_channels) != 0:
                    str_diff_channels = ", ".join(diff_channels)
                    str_millis = "{:09}".format(ts_milliseconds)
                    print(f"{ts_seconds}.{str_millis}: dt = {dt} < {dt_threshold} at {str_diff_channels}!")

        ts_seconds_old, ts_milliseconds_old = ts_seconds, ts_milliseconds

        values_older = values_old
        values_old = values

        dav = int(values[dav_idx])
        if dav == 0:
            if should_print:
                is_error = False
                eoi_val = int(values[eoi_idx])
                atn_val = int(values[atn_idx])
                dio_lines = values[ch0_idx:ch0_idx+8]
                should_print = False
            elif not is_error:
                if int(values[eoi_idx]) != eoi_val:
                    print("EOI change while low DAV at {}s!".format(values[time_idx]))
                    is_error = True
                    continue
                if int(values[atn_idx]) != atn_val:
                    print("ATN change while low DAV at {}s!".format(values[time_idx]))
                    is_error = True
                    continue
                for i, val in enumerate(values[ch0_idx:ch0_idx+8]):
                    if val != dio_lines[i]:
                        print("DIO[{}] change while low DAV at {}s!".format(i, values[time_idx]))
                        is_error = True
                        break
        else:
            should_print = True

if __name__ == "__main__":
    main()
