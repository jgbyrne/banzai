// =-=-= main.rs =-=-=
// Simple command line wrapper for `banzai::encode`
// If I make this any more complicated, it'll probably
// get its own crate...

use banzai::encode;
use std::convert;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process;

const SUCCESS: i32 = 0;
const ERR_ARGS: i32 = 1;
const ERR_FILESYSTEM: i32 = 2;
const ERR_OUTPUT: i32 = 3;

const TAGLINE: &'static str = "banzai is a libre bzip2 encoder";
const VERSION: &'static str = concat!("version alpha ", env!("CARGO_PKG_VERSION"));

fn fs_die(e: io::Error) -> ! {
    eprintln!("[filesystem error] {}", e);
    process::exit(ERR_FILESYSTEM);
}

fn synopsis_die() -> ! {
    eprintln!("{}", TAGLINE);
    eprintln!("   run 'bnz --help' for a full list of options");
    eprintln!("   run 'bnz --info' for information about this software");
    eprintln!("{}", VERSION);
    process::exit(ERR_ARGS);
}

const USAGE_MSG: &'static str = r#"
  usage: bnz [options] <input_path>

  options:
     --output <path.bz2>    specify output file
     --stdout    or   -c    output to standard out
     --keep      or   -k    keep input file

     -1 to -9               set block size (100 to 900 kB)

     --verbose   or   -v    more extensive logging

  commands:
     --help                 print this message
     --info                 print information about banzai
     --version              print version string

  notes:
     To read input from stdin, specify '-' in place of the
     input path. If neither '--output' nor '--stdout' are
     specified, the file '<input_path>.bz2' is written.
"#;

fn help_die() -> ! {
    eprintln!("{}", TAGLINE);
    eprintln!("{}", USAGE_MSG);
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

const INFO_MSG: &'static str = r#"
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
    eprintln!("{}", TAGLINE);
    eprintln!("{}", INFO_MSG);
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

fn version_die() -> ! {
    eprintln!("{}", VERSION);
    process::exit(SUCCESS);
}

fn args_error_die<S: convert::AsRef<str>>(msg: S) -> ! {
    eprintln!("{}", msg.as_ref());
    process::exit(ERR_ARGS);
}

enum ArgExpect {
    Any,
    NoArgs,
    OutPath,
}

enum Input {
    Unspecified,
    File(String),
    StdIn,
}

enum Output {
    Unspecified,
    File(String),
    StdOut,
}

struct Invocation {
    input: Input,
    output: Output,
    verbose: bool,
    keep_inf: bool,
    level: Option<usize>,
}

impl Invocation {
    fn blank() -> Self {
        Self {
            input: Input::Unspecified,
            output: Output::Unspecified,
            verbose: false,
            keep_inf: false,
            level: None,
        }
    }

    fn with_input(&mut self, input: Input) {
        match self.input {
            Input::Unspecified => {
                self.input = input;
            },
            _ => {
                args_error_die("Only one input may be specified");
            },
        }
    }

    fn with_output(&mut self, output: Output) {
        match self.output {
            Output::Unspecified => {
                self.output = output;
                return;
            },
            Output::StdOut => {
                // tolerate specifying stdout multiple times
                if let Output::StdOut = output {
                    return;
                }
            },
            Output::File(_) => {},
        }
        args_error_die("Only one output may be specified");
    }

    fn level(&self) -> usize {
        match self.level {
            Some(lvl) => lvl,
            None => 9,
        }
    }
}

fn main() {
    let args = env::args().skip(1);

    if args.len() == 0 {
        synopsis_die();
    }

    let mut invocation = Invocation::blank();
    let mut exp = ArgExpect::Any;

    for a in args {
        match exp {
            ArgExpect::Any if a.starts_with("--") => match a.as_str() {
                "--help" => help_die(),
                "--version" => version_die(),
                "--info" => info_die(),
                "--verbose" => {
                    invocation.verbose = true;
                },
                "--keep" => {
                    invocation.keep_inf = true;
                },
                "--output" => {
                    exp = ArgExpect::OutPath;
                },
                "--stdout" => {
                    invocation.with_output(Output::StdOut);
                },
                "--" => {
                    exp = ArgExpect::NoArgs;
                },
                _ => {
                    args_error_die(&format!("Unrecognised argument {}", a));
                },
            },
            ArgExpect::Any if a.starts_with('-') => match a.as_str() {
                "-" => {
                    invocation.with_input(Input::StdIn);
                },
                _ => {
                    for c in a.chars().skip(1) {
                        match c {
                            'c' => {
                                invocation.with_output(Output::StdOut);
                            },
                            'k' => {
                                invocation.keep_inf = true;
                            },
                            'v' => {
                                invocation.verbose = true;
                            },
                            n @ '0'..='9' => {
                                invocation.level = Some(n.to_digit(10).unwrap() as usize);
                            },
                            _ => {
                                args_error_die(format!("Flag '{}' is not valid", c));
                            },
                        }
                    }
                },
            },
            ArgExpect::Any | ArgExpect::NoArgs => {
                invocation.with_input(Input::File(a));
            },
            ArgExpect::OutPath => {
                if a.starts_with('-') {
                    args_error_die("Argument '--output' requires a file path");
                }
                invocation.with_output(Output::File(a));
                exp = ArgExpect::Any;
            },
        }
    }

    let mut reader: Box<dyn BufRead> = match &invocation.input {
        Input::Unspecified => args_error_die("An input must be specified"),
        Input::File(ref path) => {
            let inf = fs::File::open(path).unwrap_or_else(|err| fs_die(err));
            Box::new(BufReader::new(inf))
        },
        Input::StdIn => Box::new(BufReader::new(io::stdin())),
    };

    let writer: Box<dyn Write> = match &invocation.output {
        Output::Unspecified => {
            if let Input::File(ref inpath) = &invocation.input {
                let outf =
                    fs::File::create(&format!("{}.bz2", inpath)).unwrap_or_else(|err| fs_die(err));
                Box::new(outf)
            } else {
                Box::new(io::stdout())
            }
        },
        Output::File(ref outpath) => {
            let outf = fs::File::create(outpath).unwrap_or_else(|err| fs_die(err));
            Box::new(outf)
        },
        Output::StdOut => Box::new(io::stdout()),
    };

    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .unwrap_or_else(|err| fs_die(err));

    let input = buffer;

    let writer = BufWriter::new(writer);

    if let Err(io_err) = encode(input, writer, invocation.level()) {
        eprintln!("error writing compressed output: {}", io_err);
        process::exit(ERR_OUTPUT);
    }

    if !invocation.keep_inf {
        if let Input::File(inpath) = invocation.input {
            if let Err(io_err) = fs::remove_file(inpath) {
                eprintln!("error deleting input file: {}", io_err);
                process::exit(ERR_OUTPUT);
            }
        }
    }

    process::exit(SUCCESS);
}
