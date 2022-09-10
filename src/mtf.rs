// =-=-= mtf.rs =-=-=
// Move-to-front and RLE2 encoding

pub struct Mtf {
    pub output: Vec<u16>,
    pub num_syms: usize,
    pub freqs: Vec<usize>,
}

pub fn mtf_and_rle(buf: Vec<u8>, has_byte: Vec<bool>) -> Mtf {
    let mut names: Vec<u8> = vec![0; 256];
    let mut num_names: u16 = 0;
    for (b, present) in has_byte.iter().enumerate() {
        if *present {
            names[b] = num_names as u8;
            num_names += 1;
        }
    }

    assert!(num_names > 0);
    assert!(num_names < 257);

    let run_a = 0;
    let run_b = 1;
    let eob = num_names + 1;

    let mut freqs: Vec<usize> = vec![0; 258];
    let mut output: Vec<u16> = Vec::with_capacity(buf.len() / 3);
    let mut recency = (0..num_names).map(|s| s as u8).collect::<Vec<u8>>();

    let rle = |output: &mut Vec<u16>, freqs: &mut Vec<usize>, zero_count: usize| {
        let mut code = zero_count + 1;
        loop {
            let bit = code & 1;
            code = code >> 1;
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

    let mut i = 0;
    let mut zero_count: usize = 0;
    while let Some(b) = buf.get(i) {
        let name = names[*b as usize];
        let primary = recency[0];

        if name == primary {
            zero_count += 1;
        } else {
            if zero_count != 0 {
                rle(&mut output, &mut freqs, zero_count);
                zero_count = 0;
            }

            let mut n0 = primary;
            recency[0] = name;

            let r_iter = recency.iter_mut().enumerate().skip(1);
            for (r_i, pos) in r_iter {
                let n1 = *pos;
                *pos = n0;
                n0 = n1;

                if name == n0 {
                    output.push((r_i + 1) as u16);
                    freqs[r_i + 1] += 1;
                    break;
                }
            }
        }

        i += 1;
    }

    if zero_count != 0 {
        rle(&mut output, &mut freqs, zero_count);
    }

    output.push(eob);
    freqs[eob as usize] = 1;

    Mtf {
        output,
        num_syms: (num_names as usize) + 2,
        freqs,
    }
}
