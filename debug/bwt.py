# =-=-= bwt.py =-=-=
# A naive, but hopefully correct implementation of
# the bzip2 BWT. Accepts one line of text only.

def main():
    line = input().rstrip("\n")
    n = len(line)
    l2 = line + line
    sa = []
    for i in range(0, n*2):
        sa.append((i, l2[i:]))

    sa.sort(key = lambda i_suf: i_suf[1])

    outs = []
    ptr = -1
    for (i, suf) in sa:
        if i < n:
            if i == 0:
                ptr = len(outs)
                outs.append(line[-1])
            else:
                outs.append(line[i - 1])

    print("'" + "".join(outs) + "'")
    print(ptr)
    print(len(line))

if __name__ == "__main__":
    main()
