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

    fn push(&mut self, byte: u8) {
        assert!(self.bound > 0);
        self.buffer.push(byte);
        self.bound -= 1;
    }
}

pub fn rle_one(buf: &[u8], level: usize) -> (Vec<u8>, usize) {
    let n = buf.len();

    if n == 0 {
        return (vec![], 0);
    }

    let max_len = 100_000 * level - 1;
    let mut out = BoundedBuffer::new(max_len);

    let mut floor = 0;
    let mut i = 0;
    let mut b = buf[i];
    loop {
        match out.bound {
            0 => {
                break;
            },
            1 => {
                out.push(b);
                i += 1;
                break;
            },
            _ => {
                out.push(b);
            },
        }

        if i + 2 >= n {
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

        if b != hop || b != buf[i + 1] {
            i += 2;
            b = hop;
        } else {
            let mut run = false;

            if i > floor {
                if b == buf[i - 1] {
                    // [i-1, i, i+1, i+2] are a run
                    if out.bound < 2 {
                        i += 2;
                        break;
                    }
                    out.push(hop);

                    i += 3;
                    run = true;
                }
            }

            if !run && i + 3 < n {
                let step = buf[i + 3];
                if b == step {
                    // [i, i+1, i+2, i+3] are a run
                    if out.bound == 0 {
                        i += 2;
                        break;
                    }
                    out.push(hop);

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
                floor = i;

                if i < n {
                    b = buf[i]
                } else {
                    break;
                }
            } else {
                i += 2;
                b = hop;
            }
        }
    }

    (out.buffer, i)
}
