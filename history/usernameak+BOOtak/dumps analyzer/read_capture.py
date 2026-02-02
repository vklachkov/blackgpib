import sys


class PacketAnalyzer():
    _block_size = 0x200
    _path = None
    _next_is_send = False
    _block_num = 0

    def __init__(self, path) -> None:
        self._path = path

    def analyze(self, packet):
        if self._next_is_send:
            self._next_is_send = False
            with open(self._path, "rb") as f:
                offset = self._block_num * self._block_size
                f.seek(offset)
                block_data = f.read(self._block_size)
            
            sent_block_data = packet[7:]
            for i,b in enumerate(block_data):
                if b != sent_block_data[i]:
                    print(f"Data mismatch at {i}: sent {hex(sent_block_data[i])}, has {hex(b)} in block!")
                    break

        if len(packet) >= 6:
            if packet[0] == 0x5f and packet[1] == 0x25 and packet[2] == 0x04:
                self._next_is_send = True
                self._block_num = packet[6] * 256 + packet[5]


def parse_bits(bits) -> int:
    res = 0
    for bit in bits[::-1]:
        res <<= 1
        val = int(bit) ^ 1
        res += val
    return res


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <CSV file> <IMG file>")
        return -1

    csv_file, img_file = sys.argv[1], sys.argv[2]
    with open(csv_file) as f:
        csv_lines = f.readlines()

    header = csv_lines[0]
    header_items = header.strip("\n").split(',')

    dav_idx = header_items.index("DAV")
    atn_idx = header_items.index("ATN")
    eoi_idx = header_items.index("EOI")

    ch0_idx = header_items.index("DIO1")

    packet = []

    analyzer = PacketAnalyzer(img_file)

    should_print = True
    for line in csv_lines[1:]:
        values = line.strip("\n").split(',')

        dav = int(values[dav_idx])
        eoi = int(values[eoi_idx])
        atn = int(values[atn_idx])

        if dav == 0:
            if should_print:
                eoi_str = "eoi" if eoi == 0 else ""
                atn_str = "atn" if atn == 0 else ""
                value = parse_bits(values[ch0_idx:ch0_idx+8])
                # print("{:02x} {:4} {:4}".format(value, atn_str, eoi_str))
                packet.append(value)
                should_print = False
                if eoi == 0:
                    print(" ".join(["{:02x}".format(b) for b in packet]), values[0])
                    analyzer.analyze(packet)
                    packet = []
        else:
            should_print = True


if __name__ == "__main__":
    main()
