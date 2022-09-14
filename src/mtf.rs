// =-=-= mtf.rs =-=-=
// Move-to-front and RLE2 encoding for bzip2

use std::mem;

// Mtf struct contains information needed for huffman coding
pub struct Mtf {
    pub output: Vec<u16>,
    pub num_syms: usize,
    pub freqs: [usize; 258],
}

// Perform MTF and RLE2 passes in tandem
pub fn mtf_and_rle(buf: Vec<u8>, has_byte: Vec<bool>) -> Mtf {
    // We may not have all 256 byte values, so we
    // sequentially name the bytes that are present
    let mut names = [0; 256];
    let mut num_names: u16 = 0;
    for (b, present) in has_byte.iter().enumerate() {
        if *present {
            names[b] = num_names;
            num_names += 1;
        }
    }

    assert!(0 < num_names && num_names < 257);

    // Special symbol values
    let run_a = 0;
    let run_b = 1;
    let eob = num_names + 1;

    // Frequency table is used for huffman coding
    let mut freqs = [0; 258];

    // Output buffer length will never exceed (buf.len() + 1)
    let mut output: Vec<u16> = Vec::with_capacity(buf.len() / 3);

    // Recent names stack for MTF encoding
    let mut recency = [0; 256];
    for n in 0..num_names {
        recency[n as usize] = n;
    }

    // Encode a specified length run of zeroes with runa/runb
    let rle = |output: &mut Vec<u16>, freqs: &mut [usize], zero_count: usize| {
        let mut code = zero_count + 1;
        loop {
            let bit = code & 1;
            code >>= 1;
            if code == 0 {
                break;
            }
            match bit {
                0 => {
                    output.push(run_a);
                    freqs[run_a as usize] += 1;
                },
                _ /* 1 */ => {
                    output.push(run_b);
                    freqs[run_b as usize] += 1;
                },
            }
        }
    };

    // Main encoding loop

    let mut i = 0;
    let mut zero_count: usize = 0;
    while let Some(i_byte) = buf.get(i) {
        let i_name = names[*i_byte as usize];
        let primary = recency[0];

        /* is `i_name` the most recent name? */
        if i_name == primary {
            zero_count += 1;
        } else {
            if zero_count != 0 {
                /* encode preceding zero-run */
                rle(&mut output, &mut freqs, zero_count);
                zero_count = 0;
            }

            /* shuffle entries in recency list as far as `i_name` */
            let mut n0 = primary;
            let r_iter = recency.iter_mut().enumerate().skip(1);
            for (r_i, pos) in r_iter {
                mem::swap(pos, &mut n0);
                if i_name == n0 {
                    /* output symbol corresponding to index of `i_name` */
                    /* symbols are one larger than corrresponding indices */
                    output.push((r_i + 1) as u16);
                    freqs[r_i + 1] += 1;
                    break;
                }
            }

            /* replace front of recency list with `i_name` */
            recency[0] = i_name;
        }

        i += 1;
    }

    if zero_count != 0 {
        /* encode trailing zero-run */
        rle(&mut output, &mut freqs, zero_count);
    }

    // Blocks must always conclude with EOB
    output.push(eob);
    freqs[eob as usize] = 1;

    Mtf {
        output,
        /* a symbol for every name except zero, plus runa, runb, and eob */
        num_syms: (num_names as usize) + 2,
        freqs,
    }
}

#[cfg(tests)]
mod tests {
    use crate::mtf;

    //  Test case is Copyright 2015 Joe Tsai

    // has_byte for testing
    fn has_byte(buf: &Vec<u8>) -> Vec<bool> {
        let mut has_byte = vec![false; 256];
        for b in buf {
            has_byte[*b as usize] = true;
        }
        has_byte
    }

    #[test]
    fn smoke_test() {
        let test: Vec<u8> = vec![
            153, 45, 45, 38, 135, 179, 26, 154, 165, 170, 170, 170, 170, 18, 109, 240, 174, 150,
            87, 164, 30, 30, 30, 30, 30, 30, 30, 148, 190, 10, 60, 13, 13, 13, 13, 13, 6, 81, 200,
            13, 225, 32, 17, 43, 22, 179, 13, 13, 17, 236, 236, 236, 236, 236, 236, 236, 121, 211,
            2, 211, 185, 54, 16, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            50, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 40,
        ];
        let has_byte = has_byte(&test);

        let mtf = mtf::mtf_and_rle(test, has_byte);

        let expected: Vec<u16> = vec![
            27, 17, 0, 15, 25, 33, 15, 29, 31, 32, 0, 0, 17, 28, 40, 34, 33, 31, 34, 25, 1, 1, 34,
            36, 23, 33, 25, 1, 0, 25, 34, 37, 4, 39, 32, 31, 34, 33, 26, 7, 0, 5, 40, 1, 1, 38, 40,
            34, 2, 40, 40, 38, 38, 0, 1, 1, 0, 40, 2, 0, 1, 1, 0, 40, 41,
        ];

        assert!(mtf.output == expected);
    }
}
