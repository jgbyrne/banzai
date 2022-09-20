#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io;

extern crate banzai;

fuzz_target!(|data: &[u8]| {
    let reader = io::BufReader::new(data);
    banzai::encode(reader, io::BufWriter::new(io::sink()), 9).unwrap();
});
