mod bwt;
mod crc32;
mod huffman;
mod mtf;
mod rle;

mod out;
use out::OutputStream;

use std::convert;
use std::io;

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
    Ok(())
}

fn write_stream_footer<W: io::Write>(output: &mut OutputStream<W>, crc: u32) -> io::Result<()> {
    output.write_bytes(&[0x17, 0x72, 0x45, 0x38, 0x50, 0x90])?;
    output.write_bytes(&crc.to_be_bytes())
}

/// bzip2 encode a buffer and write the output to a `BufWriter`
///
/// `level` must be in `1..=9` and describes the block size.
/// That is: the block size is `level * 100_000`.
/// The usual default is `9`.
///
/// Returns the number of input bytes encoded.
pub fn encode<W, I>(input: I, writer: io::BufWriter<W>, level: usize) -> io::Result<usize>
where
    I: convert::AsRef<[u8]>,
    W: io::Write,
{
    assert!(1 <= level && level <= 9);
    let input = input.as_ref();
    let mut output = OutputStream::new(writer);

    write_stream_header(&mut output, level)?;

    let mut stream_crc: u32 = 0;

    let mut consumed = 0;
    while consumed < input.len() {
        let in_slice = &input[consumed..];

        let (rle_buf, block_consumed) = rle::rle_one(in_slice, level);

        let mut sum_buf = in_slice[..block_consumed].to_vec();
        let block_crc = crc32::checksum(&mut sum_buf);

        stream_crc = block_crc ^ ((stream_crc << 1) | (stream_crc >> 31));

        let bwt_out = bwt::bwt(rle_buf);

        write_block_header(&mut output, block_crc, bwt_out.ptr)?;
        write_sym_map(&mut output, &bwt_out.has_byte)?;

        let mtf_out = mtf::mtf_and_rle(bwt_out.bwt, bwt_out.has_byte);

        huffman::encode(&mut output, mtf_out)?;

        consumed += block_consumed;
    }

    write_stream_footer(&mut output, stream_crc)?;
    output.close()?;

    Ok(consumed)
}
