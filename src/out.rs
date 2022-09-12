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
        assert!(self.strand_bits < 8);
        assert!(num_bits <= 8);
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
        assert!(num_bits <= 32);
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
        assert!(n_bytes > 0);

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
