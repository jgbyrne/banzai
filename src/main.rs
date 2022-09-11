mod bwt;
mod crc32;
mod huffman;
mod mtf;
mod rle;

mod out;
use out::OutputStream;

use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::process;

use crc::Crc;

fn write_stream_header<W: io::Write>(output: &mut OutputStream<W>, level: usize) -> io::Result<()> {
    assert!(level >= 1 && level <= 9);
    let level_byte = char::from_digit(level as u32, 10).unwrap() as u8;
    output.write_bytes(&[0x42, 0x5A, 0x68, level_byte])
}

fn write_block_header<W: io::Write>(
    output: &mut OutputStream<W>,
    crc: u32,
    ptr: usize,
) -> io::Result<()> {
    output.write_bytes(&[0x31, 0x41, 0x59, 0x26, 0x53, 0x59])?;
    output.write_bytes(&crc.to_be_bytes())?;
    output.write_bits(0, 1)?;

    let ptr_bytes = ptr.to_be_bytes();
    let slice_left = ptr_bytes.len() - 3;
    output.write_bytes(&ptr.to_be_bytes()[slice_left..])
}

fn write_sym_map<W: io::Write>(
    output: &mut OutputStream<W>,
    has_byte: &Vec<bool>,
) -> io::Result<()> {
    let mut sector_map: u16 = 0;
    let mut sectors: Vec<u16> = vec![];

    for a in 0..16 {
        sector_map <<= 1;
        let mut sector = 0;
        for b in 0..16 {
            sector <<= 1;
            let byte = (a << 4) | b;
            if has_byte[byte] {
                sector |= 1;
            }
        }
        if sector != 0 {
            sector_map |= 1;
            sectors.push(sector);
        }
    }
    assert!(sectors.len() > 0);
    output.write_bytes(&sector_map.to_be_bytes())?;
    for s in sectors {
        output.write_bytes(&s.to_be_bytes())?;
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let mut buffer = Vec::new();
    let path = match args.next() {
        Some(path) => {
            match fs::File::open(&path) {
                Ok(mut file) => {
                    if let Err(e) = file.read_to_end(&mut buffer) {
                        eprintln!("[error] {}", e);
                        process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("[error] {}", e);
                    process::exit(1);
                },
            }
            path
        },
        None => {
            eprintln!("banzai <file_to_encode>");
            process::exit(1);
        },
    };

    let buffer = buffer;
    let level = 1;

    let writer = io::BufWriter::new(fs::File::create(&format!("{}.bz2", path)).unwrap());
    let mut output = OutputStream::new(writer);

    write_stream_header(&mut output, level).unwrap();

    // Build and write one block for the moment

    let mut sum_buf = buffer.clone();
    let sum = crc32::checksum(&mut sum_buf);

    let (buffer, buffer_len) = rle::rle_one(&buffer, level);

    let bwt_out = bwt::bwt(buffer);

    write_block_header(&mut output, sum, bwt_out.ptr).unwrap();

    write_sym_map(&mut output, &bwt_out.has_byte);

    let mtf_out = mtf::mtf_and_rle(bwt_out.bwt, bwt_out.has_byte);

    huffman::encode(&mut output, mtf_out).unwrap();

    output
        .write_bytes(&[0x17, 0x72, 0x45, 0x38, 0x50, 0x90])
        .unwrap();
    output.write_bytes(&[0x00, 0x00, 0x00, 0x00]).unwrap();

    output.close().unwrap();
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn bwt_basic() {
        let test = "SIX.MIXED.PIXIES.SIFT.SIXTY.PIXIE.DUST.BOXES";

        let bwt = bwt::bwt(String::from(test).into_bytes());
        let bwt_str = String::from_utf8(bwt.bwt).unwrap();

        assert!(bwt_str == "TEXYDST.E.IXIXIXXSSMPPS.B..E.S.EUSFXDIIOIIIT");
        assert!(bwt.ptr == 29);
    }

    #[test]
    fn bitstring() {
        let mut out = Vec::new();
        let writer = io::BufWriter::new(&mut out);
        let mut output = OutputStream::new(writer);

        /* 110 */
        output.write_bits(6u8, 3).unwrap();
        /* 11001000 */
        output.write_byte(200u8).unwrap();
        /* 0 */
        output.write_bits(0u8, 1).unwrap();
        /* 11001010 11111110 10111010 10111110 */
        output.write_bytes(&[0xCA, 0xFE, 0xBA, 0xBE]).unwrap();
        /* 0000001 */
        output.write_bits(1u8, 7).unwrap();

        output.close().unwrap();

        assert!(out == [0xD9, 0x0C, 0xAF, 0xEB, 0xAB, 0xE0, 0x20]);
    }

    #[test]
    fn rle1_basic() {
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

    fn has_byte(buf: &Vec<u8>) -> Vec<bool> {
        let mut has_byte = vec![false; 256];
        for b in buf {
            has_byte[*b as usize] = true;
        }
        has_byte
    }

    #[test]
    fn mtf_basic() {
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
        println!("{:?}", mtf.output);
        assert!(mtf.output == expected);
    }
}
