// =-=-= main.rs =-=-=
// Simple command line wrapper for `banzai::encode`
// If I make this any more complicated, it'll probably
// get its own crate...

use banzai::encode;
use std::env;
use std::fs;
use std::io::{self, BufWriter, Read};
use std::process;

const SUCCESS: i32 = 0;
const ERR_ARGS: i32 = 1;
const ERR_FILESYSTEM: i32 = 2;
const ERR_OUTPUT: i32 = 3;

const VERSION: &'static str = concat!("version alpha ", env!("CARGO_PKG_VERSION"));

fn fs_die(e: io::Error) -> ! {
    eprintln!("[filesystem error] {}", e);
    process::exit(ERR_FILESYSTEM);
}

fn synopsis_die() -> ! {
    eprintln!("banzai is a libre bzip2 encoder");
    eprintln!("     usage  : banzai file_to_encode");
    eprintln!("     run 'banzai --help' for full options");
    eprintln!("{}", VERSION);
    process::exit(ERR_ARGS);
}

const USAGE_MSG: &'static str = r#"
  usage: banzai [options] <input_path>

  options:
     --output <path.bz2>    specify output file
     --stdout    or   -c    output to standard out
     --replace   or   -r    delete original file

     -1 to -9               set block size (100 to 900 kB)

     --verbose   or   -v    more extensive logging

  commands:
     --help                 print this message
     --info                 print information about banzai
     --version              print version string

  notes:
     To read input from stdin, specify '--' as the input
     path. If neither of '--output' and '--stdout' are
     specified, the default output is 'input_path.bz2'.
"#;

fn help_die() -> ! {
    eprintln!("banzai is a libre bzip2 encoder");
    eprintln!("{}", USAGE_MSG);
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

const INFO_MSG: &'static str =
r#"banzai is a libre bzip2 encoder

This program uses the SA-IS algorithm to compute
the Burrows-Wheeler Transform, while codeword lengths
for Huffman Encoding are chosen by iterative refinement.

It is asymptotically linear-time in the input and is
implemented wholly in safe, modern, and idiomatic Rust.

However, it is currently in the alpha stage and should
not be used as part of a production software system.

This software is written and maintained by Jack Byrne,
and released for public use under the MIT license.
"#;

fn info_die() -> ! {
    eprintln!("{}", INFO_MSG);
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

fn version_die() -> ! {
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

enum ArgExpect {
    Any,
}

enum Input {
    None,
    StdIn,

}

fn main() {
    let args = env::args().skip(1);

    let mut path = None;

    let mut exp = ArgExpect::Any;
    for a in args {
        match exp {
            ArgExpect::Any => {
                if a.starts_with('-') {
                    match a.as_str() {
                        "--help" => help_die(),
                        "--version" => version_die(),
                        "--info" => info_die(),

                        _ => {
                            eprintln!("This is another message");
                            process::exit(ERR_ARGS);
                        }
                    }
                }
                else {
                    path = Some(a);
                }
            }
        }
    }

    let path = path.unwrap_or_else(|| synopsis_die());

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
