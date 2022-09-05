// =-=-= bwt.rs =-=-=
// Implementation of the bzip2 variant of the Burrows-Wheeler Transform

use core::ops::{Index, IndexMut};
use std::slice;

type Idx = i32;

struct Array(Vec<i32>);

impl Array {
    fn new(n: usize) -> Self {
        Self(vec![0; n])
    }

    fn len(&self) -> usize {
        self.0.len()
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

struct Data(Vec<u8>);

impl Data {
    fn iter<'d>(&'d self) -> slice::Iter<'d, u8> {
        self.0.iter()
    }
}

impl Index<Idx> for Data {
    type Output = u8;
    fn index(&self, idx: Idx) -> &Self::Output {
        &self.0[idx as usize]
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

    fn layout<I>(&mut self, data: I)
    where
        I: Iterator<Item = &'d W>,
    {
        for c in data {
            self.sizes[c.as_usize()] += 1;
            if self.sizes[c.as_usize()] == 1 {
                self.sigma.push(*c)
            }
        }
        self.sigma.sort_unstable();
    }

    fn build<I>(data: I, max_sigma_size: usize) -> Self
    where
        I: Iterator<Item = &'d W>,
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

    fn rebuild<I>(&mut self, data: I, max_sigma_size: usize)
    where
        I: Iterator<Item = &'d W>,
    {
        self.sigma.clear();
        self.sigma.reserve(max_sigma_size);
        self.sizes.clear();
        self.sizes.resize(max_sigma_size, 0);
        self.bptrs.clear();
        self.sizes.resize(max_sigma_size, 0);

        self.layout(data);
        assert!(self.sigma[self.sigma.len() - 1].as_usize() < max_sigma_size)
    }
}

#[inline]
fn tail_push<W: Word>(sa: &mut Array, buckets: &mut Buckets<W>, w: W, i: Idx) {
    let bptr = &mut buckets.bptrs[w.as_usize()];
    sa[*bptr] = i;
    if *bptr > 0 {
        *bptr -= 1
    };
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

pub fn bwt(mut data: Vec<u8>) -> (Vec<u8>, Idx) {
    if usize::BITS < 32 {
        panic!("This library does not support usize < 32");
    }

    let n: usize = data.len();

    // Establish invariant: 1 < n
    match n {
        0 => return (vec![], -1),
        1 => return (data, 0),
        _ => (),
    }

    // Establish invariant: n < Idx::MAX / 4
    // :: bzip2 block size will never exceed this
    if n >= ((Idx::MAX / 4) as usize) - 1 {
        return (vec![], -1);
    }

    // To implement wrap-around suffix sorting, we must
    // perform SA-IS on concat(data, data)
    // let buf_n = n * 2;
    // data.append(&mut data.clone());

    let buf_n = n;

    let data = Data(data);
    let mut sa = Array::new(buf_n);

    let mut buckets = Buckets::build(data.iter(), 256);

    // =-=-= SA-IS Step 1: Induced Sort all LMS-Substrings in O(n) =-=-=

    let mut lms_count = 0;

    //  Insert LMS-Substrings into respective S-Buckets

    buckets.set_ptrs_to_bucket_tails();
    let mut rtl = data.iter().rev();

    let mut i_sub = buf_n as Idx;
    let mut ty_sub = Type::L; // phantom sentinel
    let mut w_sub = rtl.next().unwrap();
    for w in rtl {
        i_sub -= 1;
        match ty_sub {
            Type::L => {
                if w < w_sub {
                    ty_sub = Type::S;
                }
            },
            Type::S => {
                if w > w_sub {
                    tail_push(&mut sa, &mut buckets, *w_sub, i_sub);
                    lms_count += 1;
                    ty_sub = Type::L;
                }
            },
        }
        w_sub = w;
    }

    if lms_count > 1 {
        // Induce L-type LMS-Prefixes from unsorted LMS-Suffixes

        buckets.set_ptrs_to_bucket_heads();

        let mut i = buf_n as Idx;
        let mut i_sup = i - 1;
        let mut i_sup2 = i - 2;

        let push_idx = if data[i_sup2] < data[i_sup] {
            !i_sup
        } else {
            i_sup
        };
        head_push(&mut sa, &mut buckets, data[i_sup], push_idx);

        for p in 0..buf_n {
            i = sa[p];
            if i > 0 {
                i_sup = i - 1;
                i_sup2 = i - 2;
                assert!(data[i_sup] >= data[i]);
                let push_idx = if i_sup2 < 0 || data[i_sup2] < data[i_sup] {
                    !i_sup
                } else {
                    i_sup
                };
                head_push(&mut sa, &mut buckets, data[i_sup], push_idx);
                sa[p] = 0;
            } else if i < 0 {
                sa[p] = !sa[p];
            }
        }

        // Induce S-type LMS-Prefixes from L-type LMS-Prefixes
        // :: LMS-Suffixes are a subset of S-type LMS-Prefixes
        // :: +tives are LMLs

        buckets.set_ptrs_to_bucket_tails();

        let mut i;
        let mut i_sup;
        let mut i_sup2;

        for p in (0..buf_n).rev() {
            i = sa[p];
            if i > 0 {
                i_sup = i - 1;
                i_sup2 = i - 2;
                assert!(data[i_sup] <= data[i]);
                let push_idx = if i_sup2 < 0 || data[i_sup2] > data[i_sup] {
                    !i_sup
                } else {
                    i_sup
                };
                tail_push(&mut sa, &mut buckets, data[i_sup], push_idx);
                sa[p] = 0;
            }
        }

        // Compress sorted LMS-Suffixes into sa[0..lms_count]

        let mut lms_count = 0;
        for p in 0..buf_n {
            if sa[p] < -1 {
                // We don't want to treat 0 as LMS: -1 = !0
                sa[lms_count] = !sa[p];
                lms_count += 1;
            }
            if p >= lms_count {
                sa[p] = Idx::MAX;
            }
        }

        // Write substring lengths into latter part of sa
        let mut rtl = data.iter().rev();

        let mut i_sub = buf_n as Idx;
        let mut ty_sub = Type::L; // phantom sentinel
        let mut w_sub = rtl.next().unwrap();

        let mut unseen_lms = lms_count;
        let mut last_lms: Idx = i_sub - 1;

        for w in rtl {
            i_sub -= 1;
            match ty_sub {
                Type::L => {
                    if w < w_sub {
                        ty_sub = Type::S;
                    }
                },
                Type::S => {
                    if w > w_sub {
                        sa[lms_count + (i_sub >> 1) as usize] = (1 + last_lms as Idx) - i_sub;

                        last_lms = i_sub;
                        unseen_lms -= 1;
                        if unseen_lms == 0 {
                            break;
                        }

                        ty_sub = Type::L;
                    }
                },
            }
            w_sub = w;
        }

        // Write reduced problem string sparsely into sa[lms_count..]

        let mut rword = 0;
        let mut prv_lms = 0;
        let mut prv_lms_len: usize = 0;
        for i in 0..lms_count {
            let cur_lms = sa[i];
            let cur_lms_len: usize = sa[lms_count + (cur_lms >> 1) as usize] as usize;

            if prv_lms != 0 {
                let eq = if (prv_lms_len == cur_lms_len) && (prv_lms_len + cur_lms_len < n) {
                    let mut offset = 0;
                    loop {
                        if offset as usize == prv_lms_len {
                            break true;
                        };
                        if data[prv_lms + offset] != data[cur_lms + offset] {
                            break false;
                        }
                        offset += 1;
                    }
                } else {
                    false
                };

                if !eq {
                    rword += 1;
                    prv_lms = cur_lms;
                    prv_lms_len = cur_lms_len;
                }
            } else {
                prv_lms = cur_lms;
                prv_lms_len = cur_lms_len;
            };
            sa[lms_count + (cur_lms >> 1) as usize] = rword;
        }

        // Compress reduced problem string into end of sa

        let mut write_ptr = buf_n - 1;
        for p in (lms_count..buf_n).rev() {
            if sa[p] != Idx::MAX {
                sa[write_ptr] = sa[p];
                write_ptr -= 1;
            }
        }

        // =-=-= SA-IS Step 2: Solve Suffix Array for Reduced Problem =-=-=

        let new_sigma_size = rword as usize + 1;
        if new_sigma_size != lms_count {
            unimplemented!();
        } else {
            // bijection between rwords and valleys
            for p in 0..lms_count {
                let w_rank = sa[buf_n - lms_count + p] as usize;
                sa[w_rank] = p as Idx;
            }
        }

        // Overwrite reduced string with indices into original string

        let mut rtl = data.iter().rev();

        let mut write_ptr = buf_n - 1;

        let mut i_sub = buf_n as Idx;
        let mut ty_sub = Type::L; // phantom sentinel
        let mut w_sub = rtl.next().unwrap();
        for w in rtl {
            i_sub -= 1;
            match ty_sub {
                Type::L => {
                    if w < w_sub {
                        ty_sub = Type::S;
                    }
                },
                Type::S => {
                    if w > w_sub {
                        sa[write_ptr] = i_sub;
                        write_ptr -= 1;
                        ty_sub = Type::L;
                    }
                },
            }
            w_sub = w;
        }

        // Overwrite reduced suffix array with the indices into the original string

        for p in 0..lms_count {
            sa[p] = sa[buf_n - lms_count + sa[p] as usize];
        }

        for p in lms_count..buf_n {
            sa[p] = 0
        }

        // Fill buckets with relatively sorted LMS-Suffixes

        buckets.set_ptrs_to_bucket_tails();

        for p in (0..lms_count).rev() {
            let lms_idx = sa[p];
            sa[p] = 0;
            tail_push(&mut sa, &mut buckets, data[lms_idx], lms_idx);
        }
    }

    // invariant here: LMS-Suffixes are sorted relative to each other in buckets

    // =-=-= SA-IS Step 3: Use sorted LMS-Suffixes to induce full Suffix Array =-=-=

    // Induce L-type LMS-suffixes from sorted LMS-Suffixes

    buckets.set_ptrs_to_bucket_heads();

    let mut i = buf_n as Idx;
    let mut i_sup = i - 1;
    let mut i_sup2 = i - 2;

    let push_idx = if data[i_sup2] < data[i_sup] {
        !i_sup
    } else {
        i_sup
    };
    head_push(&mut sa, &mut buckets, data[i_sup], push_idx);

    for p in 0..buf_n {
        i = sa[p];
        if i > 0 {
            i_sup = i - 1;
            i_sup2 = i - 2;
            assert!(data[i_sup] >= data[i]);
            let push_idx = if i_sup2 < 0 || data[i_sup2] < data[i_sup] {
                !i_sup
            } else {
                i_sup
            };
            head_push(&mut sa, &mut buckets, data[i_sup], push_idx);
        }
        sa[p] = !sa[p];
    }

    // Induce S-type LMS-Suffixes from L-type LMS-Suffixes
    // :: +tives are LMLs

    buckets.set_ptrs_to_bucket_tails();

    let mut i;
    let mut i_sup;
    let mut i_sup2;

    for p in (0..buf_n).rev() {
        i = sa[p];
        if i > 0 {
            i_sup = i - 1;
            i_sup2 = i - 2;
            assert!(data[i_sup] <= data[i]);
            let push_idx = if i_sup2 < 0 || data[i_sup2] > data[i_sup] {
                !i_sup
            } else {
                i_sup
            };
            tail_push(&mut sa, &mut buckets, data[i_sup], push_idx);
        } else if i < 0 {
            sa[p] = !sa[p];
        }
    }

    println!("{:?}", sa.0);

    unimplemented!();
}
