// =-=-= rle.rs =-=-=
// Implement the first-pass Run-Length-Encoding for bzip2

use crate::crc32;

use std::io;

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

struct InputStream<'i, I: io::BufRead> {
    reader: &'i mut I,
    seen_eof: bool,
}

impl<'i, I: io::BufRead> InputStream<'i, I> {
    fn new(reader: &'i mut I) -> Self {
        Self {
            reader,
            seen_eof: false,
        }
    }

    fn init(&mut self, raw: &mut Vec<u8>) -> usize {
        let buf = self.reader.fill_buf().unwrap();

        if buf.is_empty() {
            self.seen_eof = true;
            return raw.len();
        }

        raw.extend_from_slice(&buf);
        let buf_len = buf.len();
        self.reader.consume(buf_len);
        raw.len()
    }

    #[inline]
    fn margin_call(&mut self, raw: &mut Vec<u8>, i: usize, n: &mut usize) -> usize {
        let d = *n - i;

        // If we have less than 256 bytes of margin, this iteration could hit the
        // end of the raw buffer. So we need to fill it up if we can.
        if d < 256 {
            if self.seen_eof {
                d
            } else {
                while *n < (i + 256) {
                    let buf = self.reader.fill_buf().unwrap();
                    let l = buf.len();
                    if l == 0 {
                        self.seen_eof = true;
                        break;
                    } else {
                        raw.extend_from_slice(&buf);
                        self.reader.consume(l);
                        *n += l;
                    }
                }
                *n - i
            }
        } else {
            256
        }
    }
}

pub struct Rle {
    pub output: Vec<u8>,
    pub chk: u32,
    pub raw: Option<Vec<u8>>,
    pub consumed: usize,
}

// Apply first-pass RLE to as much of `reader` as can fit in a block
pub fn rle_one<I: io::BufRead>(reader: &mut I, mut raw: Vec<u8>, level: usize) -> Rle {
    let mut stream = InputStream::new(reader);

    let mut n = stream.init(&mut raw);

    if n == 0 {
        return Rle {
            output: vec![],
            chk: 0,
            raw: None,
            consumed: 0,
        };
    }

    /* One less than the block size maximum to allow for EOB later */
    let max_len = 100_000 * level - 1;
    let mut out = BoundedBuffer::new(max_len);

    /* Do not look for runs beneath the floor */
    let mut floor = 0;

    /* Current index and byte in `raw` */
    let mut i = 0;
    let mut b = raw[i];

    // Encode from `raw` until we hit `out.bound` or the end of `raw`
    // Approach: move through `raw` two-bytes at a time checking for runs
    loop {
        /* invariant: raw[i] == b */

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

        match stream.margin_call(&mut raw, i, &mut n) {
            0 => unreachable!(),
            1 => {
                i += 1;
                break;
            },
            2 => {
                out.push(raw[i + 1]);
                i += 2;
                break;
            },
            _ => {},
        }

        let hop = raw[i + 2];

        out.push(raw[i + 1]);

        // Check if raw[i] == raw[i + 1] == raw[i + 2]
        if b == hop && b == raw[i + 1] {
            let mut run = false;

            // Ensure run does not overlap with previous run
            // If so, check if [i-1, i, i+1, i+2] is a run
            if i > floor && b == raw[i - 1] {
                /* have we got space to encode hop and runlength? */
                if out.bound < 2 {
                    i += 2;
                    break;
                }
                out.push(hop);
                i += 3;
                run = true;
            }

            // Check if [i, i+1, i+2, i+3] is a run
            if !run && i + 3 < n {
                let step = raw[i + 3];
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
                    if let Some(r) = raw.get(i) {
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
                    b = raw[i];
                    continue;
                }
            }
        }

        // If we didn't encode a run, conclude hop
        i += 2;
        b = hop;
    }

    let remainder = if i < n { Some(raw.split_off(i)) } else { None };

    let block_crc = crc32::checksum(&mut raw);

    // Returns encoded buffer and number of input bytes encoded
    Rle {
        output: out.buffer,
        chk: block_crc,
        raw: remainder,
        consumed: i,
    }
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
