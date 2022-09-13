// =-=-= huffman.rs =-=-=
// Implementation of huffman coding for bzip2
// :: Derived largely from original bzip2 implementation
// :: (see https://sourceware.org/bzip2)

use crate::mtf;
use crate::out;
use std::cmp::Ordering;
use std::io;
use std::ops::Add;

// Current convention is to never encode a symbol longer than 17 bits
const CODEWORD_MAX_LEN: usize = 17;

// `Table` is a list s.t. table[sym] = |codeword(sym)|
type Table = Vec<u8>;

// === A singly-linked Binary Tree used for code construction ===

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
    // Create a tree with n leaves and a root, all isolated
    // :: root is id 0
    // :: leaves are id 1 to n inclusive
    // :: thus, leaf i corresponds to symbol (i - 1)
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

    // Tie two nodes of the tree together into an inner node
    // :: If this is joining the last two subtrees,
    // :: we connect them to the root node
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

    // Produce a coding `Table` of lengths
    // :: Assumes a full-constructed tree
    fn coding_lengths(&self) -> (Table, usize) {
        let mut lengths = vec![0; self.num_leaves];
        let mut max_len = 0;

        /* stack-based BFS to leaves: stack elems are (id, dist) */
        let mut stack = vec![(0, 0)];
        while let Some((cur, len)) = stack.pop() {
            let cur = &self.nodes[cur];
            if let (Some(l), Some(r)) = (cur.lchild, cur.rchild) {
                stack.push((l, len + 1));
                stack.push((r, len + 1));
            } else {
                /* if root has no children the tree wasn't finished */
                debug_assert!(cur.id != 0);

                /* symbol is one less than leaf id, so (cur.id - 1) */
                lengths[cur.id - 1] = len as u8;
                if len > max_len {
                    max_len = len;
                }
            }
        }

        (lengths, max_len)
    }

    // Ugly recursive pretty-print for debugging
    #[allow(unused)]
    fn str_node(&self, node: usize, depth: usize) -> String {
        let indent = " |".repeat(depth);
        if self.nodes[node].lchild.is_none() && self.nodes[node].rchild.is_none() {
            format!("{}{}", indent, node)
        } else {
            let lstr = match self.nodes[node].lchild {
                Some(l) => self.str_node(l, depth + 1),
                None => format!("{}-", "  ".repeat(depth + 1)),
            };

            let rstr = match self.nodes[node].rchild {
                Some(r) => self.str_node(r, depth + 1),
                None => format!("{}-", "  ".repeat(depth + 1)),
            };
            format!("{}{}\n{}\n{}", indent, node, lstr, rstr)
        }
    }

    #[allow(unused)]
    fn print(&self) {
        println!("{}", self.str_node(0, 0));
    }
}

// === Priority Queue implementation for building coding tree ===

// Elements of the `FrequencyQueue`, which correspond to `Tree` nodes,
// have a `Priority` which dictates their position in the queue.
//
// :: A priority is a pair (sum_frequency, max_dist) where
//    sum_frequency is the sum frequency of symbols in this subtree
//    and max_dist is the longest distance to a leaf in this subtree
//
// :: The sum_frequency is the main determinant, with max_dist to break ties.
//
// :: Since the `FrequencyQueue` is a min-heap:
//        a < b   =>   a is higher priority than b

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
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

struct FrequencyQueue {
    heap: Vec<(u16, Priority)>,
}

impl FrequencyQueue {
    // Create new frequency queue with `num_syms` symbols
    // :: The parameter `scaling` is initially 1, but doubles
    //    until the maximum codeword length is not exceeded
    // :: Symbol s is recorded as (s + 1) in the queue to
    //    match the id of the corresponding `TreeNode`
    fn new(num_syms: usize, freqs: &[usize], scaling: usize) -> Self {
        let mut queue = Self {
            heap: Vec::with_capacity(num_syms),
        };
        for s in 0..num_syms {
            let q_freq = (freqs[s] / scaling) + 1;
            queue.insert((s + 1) as u16, Priority(q_freq, 0));
        }
        queue
    }

    // We use 1-indexing for the heap
    // :: This allows us to traverse the heap with binary shifts

    #[inline]
    fn item(&mut self, idx: usize) -> &mut (u16, Priority) {
        &mut self.heap[idx - 1]
    }

    #[inline]
    fn read_item(&self, idx: usize) -> &(u16, Priority) {
        &self.heap[idx - 1]
    }

    // Insert `sym` with `priority` into the queue
    fn insert(&mut self, sym: u16, priority: Priority) {
        let init_idx = self.heap.len() + 1;
        self.heap.push((sym, priority));

        if init_idx == 1 {
            return;
        }

        let mut this_idx = init_idx;
        loop {
            let above_idx = this_idx >> 1;

            let (above_sym, above_priority) = *self.item(above_idx);
            if priority < above_priority {
                *self.item(this_idx) = (above_sym, above_priority);
                this_idx = above_idx;
                if this_idx == 1 {
                    break;
                }
            } else {
                break;
            }
        }
        if this_idx != init_idx {
            *self.item(this_idx) = (sym, priority);
        }
    }

    // Extract the root of the heap
    fn extract(&mut self) -> (u16, Priority) {
        let (sym, priority) = match self.heap.pop() {
            None => panic!("Tried to extract() from empty heap"),
            Some(last) => {
                if self.heap.is_empty() {
                    return last;
                }
                last
            },
        };

        let root = *self.item(1);

        *self.item(1) = (sym, priority);
        let heap_size = self.heap.len();

        let mut this_idx = 1;
        let final_idx = loop {
            let left_idx = this_idx << 1;
            if left_idx > heap_size {
                break this_idx;
            }
            let right_idx = left_idx + 1;

            let (below_idx, (below_sym, below_priority)) = {
                if right_idx <= heap_size
                    && self.read_item(right_idx).1 < self.read_item(left_idx).1
                {
                    (right_idx, *self.item(right_idx))
                } else {
                    (left_idx, *self.item(left_idx))
                }
            };

            if priority < below_priority {
                break this_idx;
            }
            *self.item(this_idx) = (below_sym, below_priority);
            this_idx = below_idx;
        };
        *self.item(final_idx) = (sym, priority);
        root
    }
}

// Build a coding table from a list of symbol frequencies
fn build_table_from_freqs(num_syms: usize, freqs: &[usize]) -> Table {
    let mut scaling = 1;

    /* Attempt iteratively, rescaling until CODEWORD_MAX_LEN is respected */
    loop {
        let mut tree = Tree::with_n_leaves(num_syms);
        let mut queue = FrequencyQueue::new(num_syms, freqs, scaling);

        loop {
            let (sym_one, priority_one) = queue.extract();
            let (sym_two, priority_two) = queue.extract();

            let parent = tree.tie(sym_one as usize, sym_two as usize);

            /* if tree is complete */
            if parent == 0 {
                break;
            }
            queue.insert(parent as u16, priority_one + priority_two);
        }

        let (lengths, max_len) = tree.coding_lengths();
        if max_len <= CODEWORD_MAX_LEN {
            break lengths;
        }
        scaling <<= 1;
    }
}

// === Huffman coding algorithm ===

// Initial pseudo-frequencies for coding tables
const INIT_LEN_HIGH: u8 = 15;
const INIT_LEN_LOW: u8 = 0;

// Number of table improvement iterations
const NUM_REFINEMENTS: u8 = 4;

// bzip2 requires a coding selection every 50 bytes
const SEGMENT_WIDTH: usize = 50;

// Encode the output of an MTF transform and write to the output stream
pub fn encode<W: io::Write>(output: &mut out::OutputStream<W>, mtf: mtf::Mtf) -> io::Result<()> {
    let input = mtf.output;
    let input_size = input.len();
    let num_syms = mtf.num_syms;

    // Between 2 and 6 tables, using same thresholds as reference implementation
    let num_tables = match num_syms {
        0..=2 => panic!("Too few symbols for huffman::encode();"),
        3..=199 => 2,
        200..=599 => 3,
        600..=1199 => 4,
        1200..=2399 => 5,
        _ => 6,
    };

    // Build initial coding tables
    // :: The approach is to assign each table a contiguous chunk of the symbol
    // :: range such that each chunk has approximately equal sum frequency,
    // :: then initialise each table with a bias for the symbols in its range

    let mut tables = vec![Table::with_capacity(num_syms); num_tables];

    let mut freq_remaining = input_size;
    let mut sym_left = 0;

    for cur_table in 0..num_tables {
        let tables_remaining = num_tables - cur_table;
        let freq_target = freq_remaining / tables_remaining;

        let mut freq_acc = 0;

        let mut sym_right = sym_left;
        loop {
            freq_acc += mtf.freqs[sym_right];
            if freq_acc >= freq_target || (sym_right + 1) == num_syms {
                break;
            }
            sym_right += 1;
        }

        // This strange check backtracks one symbol on odd internal
        // tables to push the average 'greediness' towards zero
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

    // The heart of the coding algorithm:
    // :: Iteratively:
    //      -> Determine the best table for each 50-byte segment,
    //         then recalculate the frequency list for each table
    //         based on the sum symbol frequencies from the segments
    //         for which it is the best fit.
    //
    //      -> Rebuild tables from new frequency lists
    //
    // :: On final iteration, push best table ids to selector list

    /* new sum frequencies for each table */
    let mut table_freqs: Vec<Vec<usize>> = vec![];

    for _ in 0..num_tables {
        table_freqs.push(vec![0; num_syms]);
    }

    /* populated on final iteration */
    let mut selectors: Vec<usize> = vec![];

    for it in 0..NUM_REFINEMENTS {
        let final_it = it == (NUM_REFINEMENTS - 1);

        /* zero out frequency lists on each iteration */
        if it != 0 {
            for table in &mut tables {
                for s in 0..num_syms {
                    table[s] = 0;
                }
            }
        }

        /* iterate over segments */
        let mut buf_left = 0;
        let mut buf_right;
        loop {
            buf_right = buf_left + SEGMENT_WIDTH - 1;

            /* last segment may be abbreviated */
            if buf_right >= input_size {
                buf_right = input_size - 1;
            }

            /* accumulate coding costs for each table */

            let mut best_table = 0;
            let mut best_table_cost = usize::MAX;

            for (t, table) in tables.iter().enumerate() {
                let mut cost = 0;

                for s in &input[buf_left..=buf_right] {
                    cost += table[*s as usize] as usize;
                }

                if cost < best_table_cost {
                    best_table = t;
                    best_table_cost = cost;
                }
            }

            /* segment contributes its frequency to the best table */
            for s in &input[buf_left..=buf_right] {
                table_freqs[best_table][*s as usize] += 1;
            }

            /* on final iteration build selectors list */
            if final_it {
                selectors.push(best_table);
            }

            buf_left = buf_right + 1;
            if buf_left >= input_size {
                break;
            }
        }

        /* rebuild tables with new frequency lists */
        for table in 0..num_tables {
            tables[table] = build_table_from_freqs(num_syms, &table_freqs[table]);
        }
    }

    // === Write encoded data to the output ===

    // num_tables as 3 bit integer
    output.write_bits(num_tables as u8, 3)?;

    // num_selectors as 15 bit integer
    let num_selectors = selectors.len() as u32;
    output.write_bits_u32(num_selectors, 15)?;

    // selectors list is MTF encoded
    let mut selectors_mtf = Vec::with_capacity(num_tables);
    let mut idx_codes = Vec::with_capacity(num_tables);

    for i in 0..num_tables {
        selectors_mtf.push(i);
        if i == 0 {
            idx_codes.push(0);
        } else {
            /* 'i' ones followed by a zero */
            idx_codes.push((1 << (i + 1)) - 2)
        }
    }

    for sel in selectors.iter() {
        let mut bump = selectors_mtf[0];
        if bump == *sel {
            output.write_bits(0, 1)?;
        } else {
            let mut idx = 1;
            loop {
                let stack_sel = selectors_mtf[idx];
                selectors_mtf[idx] = bump;
                if stack_sel == *sel {
                    output.write_bits(idx_codes[idx], idx + 1)?;
                    break;
                }
                bump = stack_sel;
                idx += 1;
            }
            selectors_mtf[0] = *sel;
        }
    }

    // Coding tables are delta-encoded

    let mut codings = Vec::with_capacity(num_tables);

    for table in tables {
        let mut min_len = u8::MAX;
        let mut max_len = 0;

        // Initial coding length is 5-bit integer
        output.write_bits(table[0], 5)?;

        let mut acc = table[0];
        for l in table.iter() {
            // Encode delta from previous length
            loop {
                match (*l).cmp(&acc) {
                    Ordering::Equal => {
                        /* 0 when length matches */
                        output.write_bits(0, 1)?;
                        break;
                    },
                    Ordering::Greater => {
                        /* 10 means increment */
                        output.write_bits(2, 2)?;
                        acc += 1;
                    },
                    Ordering::Less => {
                        /* 11 means decrement */
                        output.write_bits(3, 2)?;
                        acc -= 1;
                    },
                }
            }

            if *l < min_len {
                min_len = *l;
            }
            if *l > max_len {
                max_len = *l;
            }
        }

        // Construct codewords for this table
        // :: coding[sym] = (word_length, word)

        let mut coding = vec![(0, 0); num_syms];
        let mut word: u32 = 0;
        for l in min_len..=max_len {
            for s in 0..num_syms {
                if table[s as usize] == l {
                    coding[s as usize] = (l as usize, word);
                    word += 1;
                }
            }
            word <<= 1;
        }
        codings.push(coding);
    }

    // Huffman encode input buffer
    let mut sel = selectors[0];
    for (i, s) in input.iter().enumerate() {
        if i % 50 == 0 {
            sel = selectors[i / 50];
        }
        let (word_len, word) = codings[sel][*s as usize];
        output.write_bits_u32(word, word_len)?;
    }

    Ok(())
}
