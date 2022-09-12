// =-=-= main.rs =-=-=
// Simple command line wrapper for `banzai::encode`
// If I make this any more complicated, it'll probably
// get its own crate...

use banzai::encode;
use std::env;
use std::fs;
use std::io::{self, BufWriter, Read};
use std::process;

const ERR_ARGS: i32 = 1;
const ERR_FILESYSTEM: i32 = 2;
const ERR_OUTPUT: i32 = 3;

fn fs_die(e: io::Error) -> ! {
    eprintln!("[filesystem error] {}", e);
    process::exit(ERR_FILESYSTEM);
}

fn help_die() -> ! {
    eprintln!("banzai is a libre bzip2 encoder");
    eprintln!("     usage  : banzai file_to_encode");
    eprintln!("     output : file_to_encode.bz2");
    eprintln!("version pre-alpha {}", env!("CARGO_PKG_VERSION"));
    process::exit(ERR_ARGS);
}

fn main() {
    let mut args = env::args().skip(1);
    if args.len() > 1 {
        help_die()
    }
    let path = args.next().unwrap_or_else(|| help_die());

    let mut file = fs::File::open(&path).unwrap_or_else(|err| fs_die(err));

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .unwrap_or_else(|err| fs_die(err));

    let input = buffer;

    let outf = fs::File::create(&format!("{}.bz2", path)).unwrap_or_else(|err| fs_die(err));
    let writer = BufWriter::new(outf);

    let level = 9;
    if let Err(io_err) = encode(input, writer, level) {
        eprintln!("[output error] {}", io_err);
        process::exit(ERR_OUTPUT);
    }
}
