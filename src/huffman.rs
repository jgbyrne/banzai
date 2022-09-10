// =-=-= huffman.rs =-=-=
// Implementation of huffman coding for bzip2
// Largely a port of the original C implementation

use std::io;
use crate::mtf;
use crate::out;

type Table = Vec<u8>;

const INIT_LEN_HIGH: u8 = 15;
const INIT_LEN_LOW: u8 = 0;

const NUM_REFINEMENTS: u8 = 4;
const SELECTION_WIDTH: usize = 50;

fn encode<W: io::Write>(output: &mut out::OutputStream<W>, mtf: mtf::Mtf) -> io::Result<()> {
    let input = mtf.output;
    let input_size = input.len();
    let num_syms = mtf.num_syms;

    let num_tables = match num_syms {
        0..=2 => panic!("Too few symbols for huffman::encode();"),
        3..=199 => 2,
        200..=599 => 3,
        600..=1199 => 4,
        1200..=2399 => 5,
        _ => 6,
    };

    let mut tables = vec![Table::with_capacity(num_syms); num_tables];
    
    let mut freq_remaining = input_size;

    let mut sym_left = 0;
    for cur_table in 0..num_tables {
        let tables_remaining = num_tables - cur_table;
        let freq_target = freq_remaining / tables_remaining;

        let mut freq_acc = 0;

        let mut sym_right = 0;
        loop {
            freq_acc += mtf.freqs[sym_right];
            if freq_acc >= freq_target || sym_right == num_syms {
                break;
            }
            sym_right += 1;
        }

        // This strange check backtracks on odd internal tables
        // to try and neutralise the average 'greediness' 
        if sym_right > sym_left &&
           cur_table != 0 &&
           cur_table != (num_tables - 1) &&
           cur_table % 2 == 1 { 
               freq_acc -= mtf.freqs[sym_right];
               sym_right -= 1;
        }

        for s in 0..num_syms {
            tables[cur_table].push({
                if s >= sym_left && s <= sym_right {
                    INIT_LEN_HIGH
                } else {
                    INIT_LEN_LOW
                }
            });
        }

        sym_left = sym_right + 1;
        freq_remaining -= freq_acc;
    }

    let mut table_freqs: Vec<Vec<usize>> = vec![];
    let mut table_sum_lens: Vec<usize> = vec![];
    for i in 0..num_tables {
        table_freqs.push(vec![0; num_syms]);
        table_sum_lens.push(0); 
    }

    let mut selectors: Vec<usize> = vec![];

    for it in 0..NUM_REFINEMENTS {
        let last_it = it == (NUM_REFINEMENTS - 1);

        for t in 0..num_tables {
            for s in 0..num_syms {
                table_freqs[t][s] = 0;
            }
        }

        let mut buf_left = 0;
        let mut buf_right;
        loop {
            buf_right = buf_left + SELECTION_WIDTH - 1;
            if buf_right >= input_size { buf_right = input_size - 1; }

            for t in 0..num_tables {
                table_sum_lens[t] = 0;
            }

            /* this gonna be slow! */
            for s in &input[buf_left..=buf_right] {
                for t in 0..num_tables {
                    table_sum_lens[t] += tables[t][*s as usize] as usize;
                }
            }
            
            let mut best_table = 0;
            let mut best_table_sum_len = usize::MAX;
            for t in 0..num_tables {
                if table_sum_lens[t] < best_table_sum_len {
                    best_table = t;
                    best_table_sum_len = table_sum_lens[t];
                }
            }

            for s in &input[buf_left..=buf_right] {
                table_freqs[best_table][*s as usize] += 1;
            }

            if last_it {
                selectors.push(best_table);
            }

            buf_left = buf_right + 1;
            if buf_left >= input_size { break; }
        }

        // make code lengths

    }

    unimplemented!();
}
