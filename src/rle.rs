// =-=-= rle.rs =-=-=
// Implement the first-pass Run-Length-Encoding for bzip2

// A buffer that must not exceed its bound
struct BoundedBuffer {
    bound: usize,
    buffer: Vec<u8>,
}

impl BoundedBuffer {
    fn new(bound: usize) -> Self {
        Self {
            bound,
            buffer: Vec::with_capacity(bound),
        }
    }

    #[inline]
    fn push(&mut self, byte: u8) {
        debug_assert!(self.bound > 0);
        self.buffer.push(byte);
        self.bound -= 1;
    }
}

// Apply first-pass RLE to as much of `buf` as can fit in a block
pub fn rle_one(buf: &[u8], level: usize) -> (Vec<u8>, usize) {
    let n = buf.len();

    if n == 0 {
        return (vec![], 0);
    }

    /* One less than the block size maximum to allow for EOB later */
    let max_len = 100_000 * level - 1;
    let mut out = BoundedBuffer::new(max_len);

    /* Do not look for runs beneath the floor */
    let mut floor = 0;

    /* Current index and byte in `buf` */
    let mut i = 0;
    let mut b = buf[i];

    // Encode from `buf` until we hit `out.bound` or the end of `buf`
    // Approach: move through `buf` two-bytes at a time checking for runs
    loop {
        /* invariant: buf[i] == b */

        match out.bound {
            0 => {
                /* no more space left in block */
                break;
            },
            1 => {
                /* we shalln't be encoding any more runs */
                out.push(b);
                i += 1;
                break;
            },
            _ => {
                /* enough space in block to encode another run */
                out.push(b);
            },
        }

        if i + 2 >= n {
            /* not enough bytes in buffer to make hop */
            if i + 1 < n {
                out.push(buf[i + 1]);
                i += 2
            } else {
                i += 1;
            };
            break;
        }

        let hop = buf[i + 2];

        out.push(buf[i + 1]);

        // Check if buf[i] == buf[i + 1] == buf[i + 2]
        if b == hop && b == buf[i + 1] {
            let mut run = false;

            // Ensure run does not overlap with previous run
            // If so, check if [i-1, i, i+1, i+2] is a run
            if i > floor {
                if b == buf[i - 1] {
                    /* have we got space to encode hop and runlength? */
                    if out.bound < 2 {
                        i += 2;
                        break;
                    }
                    out.push(hop);
                    i += 3;
                    run = true;
                }
            }

            // Check if [i, i+1, i+2, i+3] is a run
            if !run && i + 3 < n {
                let step = buf[i + 3];
                if b == step {
                    /* have we got space to encode hop? */
                    if out.bound == 0 {
                        i += 2;
                        break;
                    }
                    out.push(hop);

                    /* have we got space to encode step and runlength? */
                    if out.bound < 2 {
                        i += 3;
                        break;
                    }
                    out.push(step);
                    i += 4;
                    run = true;
                }
            }

            if run {
                // Encode up to 251 additional repeated bytes
                let mut rep_count: u8 = 0;
                while rep_count < 251 {
                    if let Some(r) = buf.get(i) {
                        if b == *r {
                            rep_count += 1;
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                out.push(rep_count);

                /* don't look for next run inside this one */
                floor = i;

                if i >= n {
                    break;
                } else {
                    b = buf[i];
                    continue;
                }
            }
        }

        // If we didn't encode a run, conclude hop
        i += 2;
        b = hop;
    }

    // Returns encoded buffer and number of input bytes encoded
    (out.buffer, i)
}

#[cfg(test)]
mod tests {
    use crate::rle;

    // Test Cases are Copyright 2015 Joe Tsai

    #[test]
    fn smoke_test() {
        let test = b"aaabbbcccddddddeeefgghiiijkllmmmmmmmmnnoo";
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"aaabbbcccdddd\x02eeefgghiiijkllmmmm\x04nnoo");

        let test = ['a' as u8; 4];
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"aaaa\x00");

        let test = ['j' as u8; 255];
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"jjjj\xfb");

        let test = ['j' as u8; 259];
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"jjjj\xfbjjjj\x00");

        let test = ['j' as u8; 500];
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"jjjj\xfbjjjj\xf1");

        let test = b"aaaabbbbcccc";
        let (rle, ptr) = rle::rle_one(test.as_slice(), 1);
        assert!(ptr == test.len());
        assert!(&rle == b"aaaa\x00bbbb\x00cccc\x00");
    }
}
