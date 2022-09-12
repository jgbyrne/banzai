// =-=-= out.rs =-=-=
// Naive routines for writing out a bitstring

use std::io;
use std::io::Write;

pub struct OutputStream<W: io::Write> {
    writer: io::BufWriter<W>,
    strand: u8,
    strand_bits: usize,
}

impl<W: io::Write> OutputStream<W> {
    pub fn new(writer: io::BufWriter<W>) -> Self {
        Self {
            writer,
            strand: 0u8,
            strand_bits: 0,
        }
    }

    pub fn close(mut self) -> io::Result<()> {
        if self.strand_bits != 0 {
            self.writer.write(&[self.strand])?;
        }
        self.writer.flush()?;
        io::Result::Ok(())
    }

    #[inline]
    pub fn write_bits(&mut self, chunk: u8, num_bits: usize) -> io::Result<()> {
        debug_assert!(self.strand_bits < 8);
        debug_assert!(num_bits <= 8);
        let rptr = self.strand_bits + num_bits;

        if rptr < 8 {
            let shift = 8 - rptr;
            let s_chunk = chunk << shift;
            self.strand = self.strand | s_chunk;
            self.strand_bits = rptr;
        } else if rptr == 8 {
            self.writer.write(&[self.strand | chunk])?;
            self.strand = 0;
            self.strand_bits = 0;
        } else {
            let spill = rptr - 8;
            let s_chunk = chunk >> spill;
            self.writer.write(&[self.strand | s_chunk])?;
            let lshift = 8 - spill;
            self.strand = chunk << lshift;
            self.strand_bits = spill;
        }

        io::Result::Ok(())
    }

    #[inline]
    pub fn write_bits_u32(&mut self, chunk: u32, num_bits: usize) -> io::Result<()> {
        debug_assert!(num_bits <= 32);
        let bytes = chunk.to_be_bytes();

        let num_full_bytes = num_bits / 8;
        let num_rem_bits = num_bits % 8;

        let mut bptr = 3 - num_full_bytes;
        if num_rem_bits != 0 {
            self.write_bits(bytes[bptr], num_rem_bits)?;
        }
        bptr += 1;

        while bptr < 4 {
            self.write_byte(bytes[bptr])?;
            bptr += 1;
        }
        Ok(())
    }

    #[inline]
    pub fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.write_bits(byte, 8)
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        let n_bytes = bytes.len();
        debug_assert!(n_bytes > 0);

        if self.strand_bits == 0 {
            self.writer.write(bytes)?;
        } else {
            let rshift = self.strand_bits;
            let lshift = 8 - self.strand_bits;
            let mut buf = Vec::with_capacity(n_bytes + 1);
            let mut strand = self.strand;
            for b in bytes {
                buf.push((b >> rshift) | strand);
                strand = b << lshift;
            }
            self.writer.write(&buf)?;
            self.strand = strand;
        }

        io::Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::out;
    use std::io;

    #[test]
    fn bitstring() {
        let mut out = Vec::new();
        let writer = io::BufWriter::new(&mut out);
        let mut output = out::OutputStream::new(writer);

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
