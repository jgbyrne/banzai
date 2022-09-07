mod bwt;
mod mtf;
mod out;
mod rle;
use out::OutputStream;

use std::fs;
use std::io;

fn write_stream_header<W: io::Write>(output: &mut OutputStream<W>, level: usize) -> io::Result<()> {
    assert!(level >= 1 && level <= 9);
    let level_byte = char::from_digit(level as u32, 10).unwrap() as u8;
    output.write_bytes(&[0x42, 0x5A, 0x68, level_byte])
}

fn main() {
    let writer = io::BufWriter::new(fs::File::create("out.bz2").unwrap());
    let mut output = OutputStream::new(writer);

    write_stream_header(&mut output, 1).unwrap();

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
			153, 45, 45, 38, 135, 179, 26, 154, 165, 170, 170, 170, 170, 18, 109,
			240, 174, 150, 87, 164, 30, 30, 30, 30, 30, 30, 30, 148, 190, 10, 60,
			13, 13, 13, 13, 13, 6, 81, 200, 13, 225, 32, 17, 43, 22, 179, 13, 13,
			17, 236, 236, 236, 236, 236, 236, 236, 121, 211, 2, 211, 185, 54, 16,
			5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 50,
			5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 40,
		];
        let has_byte = has_byte(&test);

        let mtf = mtf::mtf_and_rle(test, has_byte);

        let expected: Vec<u16> = vec![
            27, 17, 0, 15, 25, 33, 15, 29, 31, 32, 0, 0, 17, 28, 40, 34, 33, 31,
			34, 25, 1, 1, 34, 36, 23, 33, 25, 1, 0, 25, 34, 37, 4, 39, 32, 31, 34,
			33, 26, 7, 0, 5, 40, 1, 1, 38, 40, 34, 2, 40, 40, 38, 38, 0, 1, 1, 0,
			40, 2, 0, 1, 1, 0, 40, 41
        ];
        println!("{:?}", mtf.output);
        assert!(mtf.output == expected);
    }
}
