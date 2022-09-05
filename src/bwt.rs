// =-=-= bwt.rs =-=-=
// Implementation of the bzip2 variant of the Burrows-Wheeler Transform

use core::ops::{Index, IndexMut};

type Idx = i32;

struct Array(Vec<i32>);

impl Array {
    fn new(n: usize) -> Self {
        Self(vec![0; n])
    }
}

impl Index<usize> for Array {
    type Output = i32;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl IndexMut<usize> for Array {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}

trait Word: Ord + TryInto<usize> + Copy {
    fn as_usize(&self) -> usize;
}

impl Word for u8 {
    #[inline]
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

struct Buckets<W: Word> {
    sigma: Vec<W>,
    sizes: Vec<usize>,
    bptrs: Vec<usize>,
}

impl<'d, W: Word + 'd> Buckets<W> {
    fn set_ptrs_to_bucket_heads(&mut self) {
        let mut acc = 0;
        for w in self.sigma.iter() {
            self.bptrs[w.as_usize()] = acc;
            acc += self.sizes[w.as_usize()];
        }
    }

    fn set_ptrs_to_bucket_tails(&mut self) {
        let mut acc = 0;
        for w in self.sigma.iter() {
            acc += self.sizes[w.as_usize()];
            self.bptrs[w.as_usize()] = acc - 1;
        }
    }

    fn layout<I>(&mut self, data: I) where
        I: Iterator<Item = &'d W>
    {
        for c in data {
            self.sizes[c.as_usize()] += 1;
            if self.sizes[c.as_usize()] == 1 {
                self.sigma.push(*c) 
            }
        }
        self.sigma.sort_unstable();
    }

    fn build<I>(data: I, max_sigma_size: usize) -> Self where
        I: Iterator<Item = &'d W>
    {
        let mut buckets = Self {
            sigma: Vec::with_capacity(max_sigma_size),
            sizes: vec![0; max_sigma_size],
            bptrs: vec![0; max_sigma_size],
        };

        buckets.layout(data);
        assert!(buckets.sigma[buckets.sigma.len() - 1].as_usize() < max_sigma_size);
        buckets
    }

    fn rebuild<I>(&mut self, data: I, max_sigma_size: usize) where
        I: Iterator<Item = &'d W>
    {
        self.sigma.clear(); self.sigma.reserve(max_sigma_size);
        self.sizes.clear(); self.sizes.resize(max_sigma_size, 0);
        self.bptrs.clear(); self.sizes.resize(max_sigma_size, 0);

        self.layout(data);
        assert!(self.sigma[self.sigma.len() - 1].as_usize() < max_sigma_size)
    }
}

#[inline]
fn tail_push<W: Word>(sa: &mut Array, buckets: &mut Buckets<W>, w: W, i: Idx) {
    let bptr = &mut buckets.bptrs[w.as_usize()];
    sa[*bptr] = i;
    if *bptr > 0 { *bptr -= 1 };
}

#[inline]
fn head_push<W: Word>(sa: &mut Array, buckets: &mut Buckets<W>, w: W, i: Idx) {
    let bptr = &mut buckets.bptrs[w.as_usize()];
    sa[*bptr] = i;
    *bptr += 1;
}

enum Type {
    S,
    L,
}

fn bwt(mut data: Vec<u8>) -> (Vec<u8>, Idx) {
    let n: usize = data.len();

    // Establish invariant: 1 < n 
    match n {
        0 => return (vec![], -1),
        1 => return (data, 0),
        _ => (),
    }

    // Establish invariant: n < Idx::MAX / 2
    // :: bzip2 block size will never exceed this
    if n >= ((Idx::MAX / 2) as usize) - 1 {
        return (vec![], -1)
    }

    // To implement wrap-around suffix sorting, we must 
    // perform SA-IS on concat(data, data)
    let buf_n = n * 2;
    data.append(&mut data.clone());
    let mut sa = Array::new(buf_n);

    let mut buckets = Buckets::build(data.iter(), 256);

    // =-=-= SA-IS Step 1: Induced Sort all LMS-Substrings in O(n) =-=-=

    //  Insert LMS-Substrings into respective S-Buckets

    buckets.set_ptrs_to_bucket_tails();
    let mut bck  = data.iter().rev();

    let mut i_adj = buf_n as Idx;
    let mut ty_adj = Type::L; // phantom sentinel
    let mut w_adj = bck.next().unwrap();
    for w in bck {
        i_adj -= 1;
        match ty_adj {
            Type::L => {
                if w < w_adj { ty_adj = Type::S; }
            },
            Type::S => {
                if w > w_adj {
                    tail_push(&mut sa, &mut buckets, *w_adj, i_adj);
                    ty_adj = Type::L;
                }
            }
        }
        w_adj = w;
    }

    // Induce L-type LMS-Prefixes from LMS-Substrings

    // Induce S-type LMS-Prefixes from L-type LMS-Prefixes 


    unimplemented!();

}


