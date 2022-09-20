#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io;
use xxhash_rust::xxh3::xxh3_64;

extern crate banzai;

fuzz_target!(|data: &[u8]| {
    let in_hash = xxh3_64(data);

    let mut out = Vec::with_capacity(555555); 
    {
        let reader = io::BufReader::new(data);
        let mut decomp = bzip2::write::BzDecoder::new(&mut out);
        banzai::encode(reader, io::BufWriter::new(&mut decomp), 1).unwrap();
        decomp.finish().unwrap();
    }

    let out_hash = xxh3_64(&out);

    if in_hash != out_hash { panic!("Hash mismatch"); }
});

