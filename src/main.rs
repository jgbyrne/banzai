use banzai::encode;
use std::env;
use std::fs;
use std::io::{BufWriter, Read};
use std::process;

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
    let writer = BufWriter::new(fs::File::create(&format!("{}.bz2", path)).unwrap());
    let level = 9;
    if let Err(io_err) = encode(buffer, writer, level) {
        eprintln!("Error writing compressed output: {}", io_err);
        process::exit(2);
    }
}
