mod bwt;
mod out;
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

    output.write_bytes(&[0x17, 0x72, 0x45, 0x38, 0x50, 0x90]).unwrap();
    output.write_bytes(&[0x00, 0x00, 0x00, 0x00]).unwrap();

    output.close().unwrap();
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

}

