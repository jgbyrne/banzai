// =-=-= huffman.rs =-=-=
// Implementation of huffman coding for bzip2
// Largely a port of the original C implementation

use crate::mtf;
use crate::out;
use std::io;
use std::ops::Add;

const CODEWORD_MAX_LEN: usize = 17;

struct TreeNode {
    id: usize,
    lchild: Option<usize>,
    rchild: Option<usize>,
}

struct Tree {
    nodes: Vec<TreeNode>,
    num_leaves: usize,
}

impl Tree {
    fn with_n_leaves(n: usize) -> Tree {
        let mut nodes = Vec::with_capacity(n * 2 - 1);
        nodes.push(TreeNode {
            id: 0,
            lchild: None,
            rchild: None,
        });
        let mut tree = Self {
            nodes,
            num_leaves: n,
        };
        for l in 1..=n {
            tree.nodes.push(TreeNode {
                id: l,
                lchild: None,
                rchild: None,
            });
        }
        tree
    }

    fn tie(&mut self, left: usize, right: usize) -> usize {
        if self.nodes.len() == (self.num_leaves * 2 - 1) {
            self.nodes[0].lchild = Some(left);
            self.nodes[0].rchild = Some(right);
            0
        } else {
            let inner_id = self.nodes.len();
            self.nodes.push(TreeNode {
                id: inner_id,
                lchild: Some(left),
                rchild: Some(right),
            });
            inner_id
        }
    }

    fn lengths(&self) -> (Table, usize) {
        let mut lengths = vec![0; self.num_leaves];
        let mut max_len = 0;

        let mut stack = vec![(0, 0)];
        while let Some((cur, len)) = stack.pop() {
            let cur = &self.nodes[cur];
            if let (Some(l), Some(r)) = (cur.lchild, cur.rchild) {
                stack.push((l, len + 1));
                stack.push((r, len + 1));
            } else {
                assert!(cur.id != 0);
                lengths[cur.id - 1] = len as u8;
                if len > max_len {
                    max_len = len;
                }
            }
        }

        (lengths, max_len)
    }
}

// Priority is (freq_sum, max_word_len)
#[derive(PartialEq, PartialOrd, Clone, Copy)]
struct Priority(usize, u8);

impl Add for Priority {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(
            self.0 + other.0,
            if self.1 > other.1 {
                self.1 + 1
            } else {
                other.1 + 1
            },
        )
    }
}

// Min-Heap for building coding tree
struct FrequencyQueue {
    heap: Vec<(u16, Priority)>,
}

impl FrequencyQueue {
    fn new(num_syms: usize, freqs: &Vec<usize>, scaling: usize) -> Self {
        let mut queue = Self {
            heap: Vec::with_capacity(num_syms),
        };
        for s in 0..num_syms {
            let q_freq = (freqs[s] / scaling) + 1;
            queue.insert(s as u16, Priority(q_freq, 0));
        }
        queue
    }

    fn insert(&mut self, sym: u16, priority: Priority) {
        let init_idx = self.heap.len();
        self.heap.push((sym, priority));

        if init_idx == 0 {
            return;
        }

        let mut this_idx = init_idx;
        loop {
            let above_idx = this_idx >> 1;

            let (above_sym, above_priority) = self.heap[above_idx];
            if priority < above_priority {
                self.heap[this_idx] = (above_sym, above_priority);
                this_idx = above_idx;
                if this_idx == 0 {
                    break;
                }
            } else {
                break;
            }
        }
        if this_idx != init_idx {
            self.heap[this_idx] = (sym, priority);
        }
    }

    fn extract(&mut self) -> (u16, Priority) {
        match self.heap.len() {
            0 => panic!("Tried to extract() from empty heap"),
            1 => return self.heap.pop().unwrap(),
            _ => {},
        }

        let root = self.heap[0];
        let (sym, priority) = self.heap.pop().unwrap();
        self.heap[0] = (sym, priority);
        let heap_size = self.heap.len();

        let mut this_idx = 0;
        loop {
            let left_idx = this_idx << 1;
            if left_idx >= heap_size {
                break;
            }
            let right_idx = left_idx + 1;

            let (below_idx, (below_sym, below_priority)) =
                if right_idx < heap_size && self.heap[left_idx].1 < self.heap[right_idx].1 {
                    (right_idx, self.heap[right_idx])
                } else {
                    (left_idx, self.heap[left_idx])
                };

            if priority < below_priority {
                break;
            }
            self.heap[this_idx] = (below_sym, below_priority);
            this_idx = below_idx;
        }
        root
    }

    #[inline]
    fn len(&self) -> usize {
        self.heap.len()
    }
}

fn build_table_from_freqs(num_syms: usize, freqs: &Vec<usize>) -> Table {
    let mut scaling = 1;

    loop {
        let mut tree = Tree::with_n_leaves(num_syms);
        let mut queue = FrequencyQueue::new(num_syms, freqs, 1);

        loop {
            let (sym_one, priority_one) = queue.extract();
            let (sym_two, priority_two) = queue.extract();

            let parent = tree.tie(sym_one as usize, sym_two as usize);
            if parent == 0 {
                break;
            }
            queue.insert(parent as u16, priority_one + priority_two);
        }

        let (lengths, max_len) = tree.lengths();
        if max_len <= CODEWORD_MAX_LEN {
            break lengths;
        }
        scaling = scaling << 1;
    }
}

type Table = Vec<u8>;

const INIT_LEN_HIGH: u8 = 15;
const INIT_LEN_LOW: u8 = 0;

const NUM_REFINEMENTS: u8 = 4;
const SELECTION_WIDTH: usize = 50;

pub fn encode<W: io::Write>(output: &mut out::OutputStream<W>, mtf: mtf::Mtf) -> io::Result<()> {
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
        if sym_right > sym_left
            && cur_table != 0
            && cur_table != (num_tables - 1)
            && cur_table % 2 == 1
        {
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
    for _ in 0..num_tables {
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
            if buf_right >= input_size {
                buf_right = input_size - 1;
            }

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
            if buf_left >= input_size {
                break;
            }
        }

        for table in 0..num_tables {
            tables[table] = build_table_from_freqs(num_syms, &mut table_freqs[table]);
        }
    }

    unimplemented!();
}
