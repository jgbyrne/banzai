# =-=-= rle1.py =-=-=
# An inefficient, but (I hope!) correct implementation of
# the bzip2 RLE1 step, for the sake of debugging.

# Unlike the real thing, does not concern itself with
# limiting the number of input bytes it encodes.

import sys
import binascii

def rle1(data):
    outbuf = []
    run_count = 0
    cur_run_chr = -1
    for i, b in enumerate(data):
        if b != cur_run_chr:
            if run_count >= 4:
                outbuf.append(run_count - 4)
            run_count = 1
            cur_run_chr = b
            outbuf.append(b)

        elif b == cur_run_chr:
            run_count += 1
            if run_count <= 4:
                outbuf.append(b)

            if run_count == 256:
                outbuf.append(run_count - 5)
                run_count = 1
                outbuf.append(b)

    if run_count >= 4:
        outbuf.append(run_count - 4)

    return bytes(outbuf)

def main():
    path = sys.argv[1]
    with open(path, 'rb') as inf:
        out = rle1(bytearray(inf.read()))
        #for i, b in enumerate(out):
        #    print("{} {:0X}".format(i, b))
        print(binascii.crc32(out))

if __name__ == "__main__":
    main()
