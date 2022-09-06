mod bwt;
mod out;
use std::fs;
use std::io;

fn main() {

}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn bwt_basic() {
        let test = "SIX.MIXED.PIXIES.SIFT.SIXTY.PIXIE.DUST.BOXES";
        let (bwt, start) = bwt::bwt(String::from(test).into_bytes());
        let bwt = String::from_utf8(bwt).unwrap();
        assert!(bwt == "TEXYDST.E.IXIXIXXSSMPPS.B..E.S.EUSFXDIIOIIIT");
        assert!(start == 29);
    }

    #[test]
    fn bitstring() {
        let mut out = Vec::new();
        let writer = io::BufWriter::new(&mut out);

        let mut output = out::OutputStream::new(writer);

        output.write_bits(6u8, 3).unwrap(); /* 110 */
        output.write_byte(200u8).unwrap(); /* 11001000 */
        output.write_bits(0u8, 1).unwrap(); /* 0 */
        /* 11001010 11111110 10111010 10111110 */
        output.write_bytes(&[0xCA, 0xFE, 0xBA, 0xBE]).unwrap();
        output.write_bits(1u8, 7).unwrap(); /* 0000001 */

        output.close().unwrap();

        assert!(out == [0xD9, 0x0C, 0xAF, 0xEB, 0xAB, 0xE0, 0x20]);
    }

}

