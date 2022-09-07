pub fn rle_one(buf: &[u8], level: usize) -> (Vec<u8>, usize) {
    let n = buf.len();

    if n == 0 {
        return (vec![], 0);
    }

    let max_len = 100_000 * level;

    let mut out_buf = Vec::with_capacity(max_len);
    let mut margin = max_len;

    let mut i = 0;
    let mut b = buf[i];
    while margin > 0 {
        out_buf.push(b);
        margin -= 1;

        if margin == 0 {
            i += 1;
            break;
        }

        if i + 2 >= n {
            if i + 1 < n {
                out_buf.push(buf[i + 1]);
                i += 2
            } else {
                i += 1;
            };
            break;
        }

        let hop = buf[i + 2];
        out_buf.push(buf[i + 1]);
        margin -= 1;

        if b != hop || b != buf[i + 1] {
            i += 2;
            b = hop;
        } else {
            let mut run = false;

            if i > 0 {
                if b == buf[i - 1] {
                    // [i-1, i, i+1, i+2] are a run
                    if margin < 2 {
                        i += 2;
                        break;
                    }
                    out_buf.push(hop);
                    margin -= 1;

                    i += 3;
                    run = true;
                }
            }

            if i + 3 < n {
                let step = buf[i + 3];
                if b == step {
                    // [i, i+1, i+2, i+3] are a run
                    if margin == 0 {
                        i += 2;
                        break;
                    }
                    out_buf.push(hop);
                    margin -= 1;

                    if margin < 2 {
                        i += 3;
                        break;
                    }
                    out_buf.push(step);
                    margin -= 1;

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
                out_buf.push(rep_count);
                margin -= 1;

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

    (out_buf, i)
}
