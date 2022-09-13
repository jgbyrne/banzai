// =-=-= bwt.rs =-=-=
// Implementation of the bzip2 variant of the Burrows-Wheeler Transform

use core::ops::{Index, IndexMut};
use std::slice;

type Idx = i32;

struct Array<'a>(&'a mut [Idx]);

impl<'r, 'a: 'r> Array<'a> {
    fn init(inner: &'a mut [Idx]) -> Self {
        Self(inner)
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn split(&'r mut self, n: usize) -> (Array<'r>, Data<'r, u32>) {
        let (sa, data) = self.0.split_at_mut(self.0.len() - n);
        let data = unsafe {
            let data_ptr = data.as_mut_ptr() as *mut u32;
            slice::from_raw_parts_mut(data_ptr, n)
        };
        for suf in sa.iter_mut() {
            *suf = 0;
        }
        (Array(&mut sa[..n]), Data(data))
    }
}

impl<'a> Index<usize> for Array<'a> {
    type Output = Idx;
    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl<'a> IndexMut<usize> for Array<'a> {
    #[inline]
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

impl Word for u32 {
    #[inline]
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

// Vec newtype that allows Idx indexing
struct Data<'d, W>(&'d mut [W]);

impl<'d, W> Data<'d, W>
where
    W: Word,
{
    #[inline]
    fn iter(&'d self) -> slice::Iter<'d, W> {
        self.0.iter()
    }

    #[inline]
    fn len(&'d self) -> usize {
        self.0.len()
    }

    fn inner(self) -> &'d mut [W] {
        self.0
    }

    #[inline]
    fn substrings_equal(&'d self, idx_a: usize, idx_b: usize, len: usize) -> bool {
        self.0[idx_a..(idx_a + len)] == self.0[idx_b..(idx_b + len)]
    }
}

impl<'d, W> Index<Idx> for Data<'d, W> {
    type Output = W;
    #[inline]
    fn index(&self, idx: Idx) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<'d, W> Index<usize> for Data<'d, W> {
    type Output = W;
    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

struct Buckets<W: Word> {
    sigma: Vec<W>,
    sizes: Vec<u32>,
    bptrs: Vec<u32>,
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
        self.bptrs.resize(max_sigma_size, 0);

        self.layout(data);
        assert!(self.sigma[self.sigma.len() - 1].as_usize() < max_sigma_size)
    }
}

#[inline]
fn tail_push<W: Word>(sa: &mut Array, buckets: &mut Buckets<W>, w: W, i: Idx) {
    let bptr = &mut buckets.bptrs[w.as_usize()];
    debug_assert!((*bptr as usize) < sa.len());
    sa[*bptr as usize] = i;
    // On the last insertion this will underflow zero
    // This is hot code so allow wrapping instead of checking the case
    *bptr = (*bptr).wrapping_sub(1);
}

#[inline]
fn head_push<W: Word>(sa: &mut Array, buckets: &mut Buckets<W>, w: W, i: Idx) {
    let bptr = &mut buckets.bptrs[w.as_usize()];
    debug_assert!((*bptr as usize) < sa.len());
    sa[*bptr as usize] = i;
    *bptr += 1;
}

enum Type {
    S,
    L,
}

fn induced_sort_fwd<W: Word>(data: &Data<W>, sa: &mut Array, buckets: &mut Buckets<W>, wipe: bool) {
    let n = sa.len();
    buckets.set_ptrs_to_bucket_heads();

    let mut i = n as Idx;
    let mut i_sup = i - 1;
    let mut i_sup2 = i - 2;

    /* Simulate sentinel by pushing data[n - 1] */
    let push_idx = if data[i_sup2] < data[i_sup] {
        !i_sup
    } else {
        i_sup
    };
    head_push(sa, buckets, data[i_sup], push_idx);

    for p in 0..n {
        i = sa[p];
        if i > 0 {
            i_sup = i - 1;
            i_sup2 = i - 2;
            debug_assert!(data[i_sup] >= data[i]);
            let push_idx = if i_sup2 < 0 || data[i_sup2] < data[i_sup] {
                !i_sup
            } else {
                i_sup
            };
            head_push(sa, buckets, data[i_sup], push_idx);
            if wipe {
                sa[p] = 0
            } else {
                sa[p] = !sa[p]
            };
        } else if i < 0 {
            sa[p] = !sa[p];
        }
    }
}

fn induced_sort_bck<W: Word>(
    data: &Data<W>,
    sa: &mut Array,
    buckets: &mut Buckets<W>,
    wipe: bool,
    unflip: bool,
) {
    let n: usize = data.len();
    buckets.set_ptrs_to_bucket_tails();

    let mut i;
    let mut i_sup;
    let mut i_sup2;

    for p in (0..n).rev() {
        i = sa[p];
        if i > 0 {
            i_sup = i - 1;
            i_sup2 = i - 2;
            debug_assert!(data[i_sup] <= data[i]);
            let push_idx = if i_sup2 < 0 || data[i_sup2] > data[i_sup] {
                !i_sup
            } else {
                i_sup
            };
            tail_push(sa, buckets, data[i_sup], push_idx);
            if wipe {
                sa[p] = 0
            };
        } else if unflip && i < 0 {
            sa[p] = !sa[p];
        }
    }
}

fn encode_reduced<W: Word>(data: &Data<W>, sa: &mut Array) -> (usize, usize) {
    let n: usize = data.len();

    #[inline]
    fn lookup_index(lms_count: usize, lms_idx: Idx) -> usize {
        lms_count + (lms_idx >> 1) as usize
    }

    // Compress sorted LMS-Substrings into sa[0..lms_count]
    let mut lms_count = 0;
    for p in 0..n {
        if sa[p] < !0 {
            /* exclude zero suffix */
            sa[lms_count] = !sa[p];
            lms_count += 1;
        }
        if p >= lms_count {
            sa[p] = Idx::MAX;
        }
    }

    // Determine LMS-Substring lengths and write into lookup indices
    let mut rtl = data.iter().rev();

    /* phantom sentinel */
    let mut i_sub = n as Idx;
    let mut ty_sub = Type::L;
    let mut w_sub = rtl.next().unwrap();

    let mut unseen_lms = lms_count;
    let mut last_lms: Idx = i_sub - 1;

    /* initially: w_sub = data[n-1], w = data[n-2] */
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
                    /* w_sub is LMS: write the substring length into its lookup index */
                    sa[lookup_index(lms_count, i_sub)] = (1 + last_lms as Idx) - i_sub;

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

    // In-place map LMS-Substrings to Lexical Names at lookup indices
    let mut rword: u32 = 0;

    let mut prv_lms: usize = 0; /* use zero as null since Idx 0 is never LMS */
    let mut prv_lms_len: usize = 0;
    for i in 0..lms_count {
        let cur_lms = sa[i];
        let lms_lookup = lookup_index(lms_count, cur_lms);

        let cur_lms_len = sa[lms_lookup] as usize;
        let cur_lms = cur_lms as usize;

        let eq = if prv_lms != 0 {
            if (prv_lms_len == cur_lms_len) && (prv_lms_len + cur_lms_len < n) {
                data.substrings_equal(prv_lms, cur_lms, prv_lms_len)
            } else {
                false
            }
        } else {
            false
        };

        if !eq {
            if prv_lms != 0 {
                rword += 1;
            }
            prv_lms = cur_lms;
            prv_lms_len = cur_lms_len;
        }

        sa[lms_lookup] = rword as Idx;
    }

    // Compress lexical names to form reduced string at end of array
    let mut write_ptr = n - 1;
    for p in (lms_count..n).rev() {
        if sa[p] != Idx::MAX {
            sa[write_ptr] = sa[p];
            write_ptr -= 1;
        }
    }

    (lms_count, rword as usize + 1)
}

fn decode_reduced<W: Word>(data: &Data<W>, sa: &mut Array, lms_count: usize) {
    let n: usize = data.len();

    // Overwrite Reduced Problem string with LMS indices
    let mut rtl = data.iter().rev();
    let mut write_ptr = n - 1;

    /* phantom sentinel */
    let mut i_sub = n as Idx;
    let mut ty_sub = Type::L;
    let mut w_sub = rtl.next().unwrap();

    /* initially: w_sub = data[n-1], w = data[n-2] */
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
                    /* w_sub is LMS */
                    sa[write_ptr] = i_sub;
                    write_ptr -= 1;
                    ty_sub = Type::L;
                }
            },
        }
        w_sub = w;
    }

    // Dereference reduced suffixes and overwrite with corresponding LMS indices
    for p in 0..lms_count {
        sa[p] = sa[n - lms_count + sa[p] as usize]
    }

    // Zero-fill everything after LMS-Suffix indices
    for p in lms_count..n {
        sa[p] = 0;
    }
}

fn sais(sigma_size: usize, data: Data<u32>, mut sa: Array, buckets: &mut Buckets<u32>) {
    let n: usize = data.len();
    assert!(n > 1);

    // =-=-= SA-IS Step 1: Induced Sort all LMS-Substrings in O(n) =-=-=

    let mut lms_count = 0;

    // Insert LMS-Substrings into respective S-Buckets

    buckets.set_ptrs_to_bucket_tails();
    let mut rtl = data.iter().rev();

    /* phantom sentinel */
    let mut i_sub = n as Idx;
    let mut ty_sub = Type::L;
    let mut w_sub = rtl.next().unwrap();

    /* initially: w_sub = data[n-1], w = data[n-2] */
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
                    /* w_sub is LMS */
                    tail_push(&mut sa, buckets, *w_sub, i_sub);
                    lms_count += 1;
                    ty_sub = Type::L;
                }
            },
        }
        w_sub = w;
    }

    /* Number of LMS suffixes is provably less than |data|/2 */
    assert!(lms_count <= (n >> 1));

    /* If we don't have multiple LMS-Suffixes we can skip to Step 3 */
    if lms_count > 1 {
        // Induced Sort Fwd: {unsorted LMS-Suffixes} => {L-Type LMS-Prefixes}
        induced_sort_fwd(&data, &mut sa, buckets, true);

        // :: invariant :: Leftmost L-Type LMS-Prefixes are +tive, all else are zero

        // Induced Sort Bck: {L-Type LMS-Prefixes} => {S-Type LMS-Prefixes}
        induced_sort_bck(&data, &mut sa, buckets, true, false);

        // :: invariant :: Leftmost S-Type LMS-Prefixes (and Idx 0) are -tive, all else are zero

        // Construct reduced problem string at end of array
        let (lms_count, new_sigma_size) = encode_reduced(&data, &mut sa);

        // =-=-= SA-IS Step 2: Solve Suffix Array for Reduced Problem =-=-=

        if new_sigma_size != lms_count {
            let (rsa, rdata) = sa.split(lms_count);
            buckets.rebuild(rdata.iter(), new_sigma_size);
            sais(new_sigma_size, rdata, rsa, buckets);
        } else {
            /* there is a bijection between rwords and LMS-suffixes */
            for p in 0..lms_count {
                let w_rank = sa[n - lms_count + p] as usize;
                sa[w_rank] = p as Idx;
            }
        }

        // Convert reduced solution into sorted LMS-Suffixes
        decode_reduced(&data, &mut sa, lms_count);

        // :: invariant :: LMS-Suffixes are in sorted order in sa[0..lms_count]

        buckets.rebuild(data.iter(), sigma_size);
        buckets.set_ptrs_to_bucket_tails();

        // Space out LMS-Suffixes into buckets
        for p in (0..lms_count).rev() {
            let lms_idx = sa[p];
            sa[p] = 0;
            tail_push(&mut sa, buckets, data[lms_idx], lms_idx);
        }
    }

    // =-=-= SA-IS Step 3: Use sorted LMS-Suffixes to induce full Suffix Array =-=-=
    // :: invariant :: LMS-Suffixes are all bucketed and sorted w.r.t each other

    // Induced Sort Fwd: {LMS-Suffixes} => {L-Type Suffixes}
    induced_sort_fwd(&data, &mut sa, buckets, false);

    // Induced Sort Bck: {L-Type Suffixes} => {S-Type Suffixes}
    induced_sort_bck(&data, &mut sa, buckets, false, true);
}

pub struct Bwt {
    pub bwt: Vec<u8>,
    pub ptr: usize,
    pub has_byte: Vec<bool>,
}

pub fn bwt(mut input: Vec<u8>) -> Bwt {
    if usize::BITS < 32 {
        panic!("This library does not support usize < 32");
    }

    let n: usize = input.len();
    let mut has_byte = vec![false; 256];

    // Establish invariant: 1 < n
    match n {
        0 => {
            return Bwt {
                bwt: vec![],
                ptr: usize::MAX,
                has_byte,
            };
        },
        1 => {
            has_byte[input[0] as usize] = true;
            return Bwt {
                bwt: input,
                ptr: 0,
                has_byte,
            };
        },
        _ => (),
    }

    // Establish invariant: n < Idx::MAX / 4
    // :: bzip2 block size will never exceed this
    if n >= ((Idx::MAX / 4) as usize) - 1 {
        return Bwt {
            bwt: vec![],
            ptr: usize::MAX,
            has_byte,
        };
    }

    // To implement wrap-around suffix sorting, we must
    // perform SA-IS on concat(data, data)
    let buf_n = n * 2;
    input.append(&mut input.clone());

    let data = Data(&mut input);
    let mut array = vec![0; buf_n];
    let mut sa = Array::init(&mut array);

    let mut buckets = Buckets::build(data.iter(), 256);

    // =-=-= SA-IS Step 1: Induced Sort all LMS-Substrings in O(n) =-=-=

    let mut lms_count = 0;

    //  Insert LMS-Substrings into respective S-Buckets

    buckets.set_ptrs_to_bucket_tails();
    let mut rtl = data.iter().rev();

    let mut i_sub = buf_n as Idx;
    let mut ty_sub = Type::L; // phantom sentinel
    let mut w_sub = rtl.next().unwrap();
    has_byte[*w_sub as usize] = true;
    for w in rtl {
        has_byte[*w as usize] = true;
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

    assert!(lms_count <= (buf_n >> 1));

    if lms_count > 1 {
        // Induced Sort Fwd: {unsorted LMS-Suffixes} => {L-Type LMS-Prefixes}
        induced_sort_fwd(&data, &mut sa, &mut buckets, true);

        // :: invariant :: Leftmost L-Type LMS-Prefixes are +tive, all else are zero

        // Induced Sort Bck: {L-Type LMS-Prefixes} => {S-Type LMS-Prefixes}
        induced_sort_bck(&data, &mut sa, &mut buckets, true, false);

        // :: invariant :: Leftmost S-Type LMS-Prefixes (and Idx 0) are -tive, all else are zero

        // Construct reduced problem string at end of array
        let (lms_count, new_sigma_size) = encode_reduced(&data, &mut sa);

        // =-=-= SA-IS Step 2: Solve Suffix Array for Reduced Problem =-=-=

        if new_sigma_size != lms_count {
            let (rsa, rdata) = sa.split(lms_count);
            let mut rbuckets = Buckets::build(rdata.iter(), new_sigma_size);
            sais(new_sigma_size, rdata, rsa, &mut rbuckets);
        } else {
            /* there is a bijection between rwords and LMS-suffixes */
            for p in 0..lms_count {
                let w_rank = sa[buf_n - lms_count + p] as usize;
                sa[w_rank] = p as Idx;
            }
        }

        // Convert reduced solution into sorted LMS-Suffixes
        decode_reduced(&data, &mut sa, lms_count);

        // Fill buckets with relatively sorted LMS-Suffixes
        buckets.set_ptrs_to_bucket_tails();

        for p in (0..lms_count).rev() {
            let lms_idx = sa[p];
            sa[p] = 0;
            tail_push(&mut sa, &mut buckets, data[lms_idx], lms_idx);
        }
    }

    // invariant here: LMS-Suffixes are sorted relative to each other in buckets

    // =-=-= SA-IS Step 3: Use sorted LMS-Suffixes to induce Burrows-Wheeler Transform =-=-=

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
            debug_assert!(data[i_sup] >= data[i]);
            if (i as usize) < n {
                sa[p] = !(data[i_sup] as Idx)
            } else {
                sa[p] = !256
            };
            let push_idx = if i_sup2 < 0 || data[i_sup2] < data[i_sup] {
                !i_sup
            } else {
                i_sup
            };
            head_push(&mut sa, &mut buckets, data[i_sup], push_idx);
        } else if i < 0 {
            sa[p] = !sa[p];
        }
    }

    // Induce S-type LMS-Suffixes from L-type LMS-Suffixes
    // :: +tives are LMLs

    buckets.set_ptrs_to_bucket_tails();

    let mut i;
    let mut i_sup;
    let mut i_sup2;

    let mut start_suffix = usize::MAX;

    for p in (0..buf_n).rev() {
        i = sa[p];
        if i > 0 {
            i_sup = i - 1;
            i_sup2 = i - 2;
            debug_assert!(data[i_sup] <= data[i]);
            sa[p] = if (i as usize) < n {
                data[i_sup] as Idx
            } else {
                256
            };
            let push_idx = if i_sup2 < 0 {
                0
            } else if data[i_sup2] > data[i_sup] {
                if (i_sup as usize) < n {
                    !(data[i_sup2] as Idx)
                } else {
                    !256
                }
            } else {
                i_sup
            };
            tail_push(&mut sa, &mut buckets, data[i_sup], push_idx);
        } else if i < 0 {
            sa[p] = !sa[p];
        } else {
            start_suffix = p;
        }
    }

    let data = data.inner();

    let mut start_ptr = usize::MAX;
    let mut j = n;
    for p in 0..buf_n {
        if p == start_suffix {
            data[j] = data[n - 1];
            start_ptr = j - n;
            j += 1;
        } else {
            let w = sa[p];
            if w < 256 {
                data[j] = w as u8;
                j += 1;
            }
        }
    }

    Bwt {
        bwt: input.split_off(n),
        ptr: start_ptr,
        has_byte,
    }
}

#[cfg(test)]
mod tests {
    use crate::bwt;

    // Test case is Copyright 2015 Joe Tsai

    #[test]
    fn smoke_test() {
        let test = "SIX.MIXED.PIXIES.SIFT.SIXTY.PIXIE.DUST.BOXES";

        let bwt = bwt::bwt(String::from(test).into_bytes());
        let bwt_str = String::from_utf8(bwt.bwt).unwrap();

        assert!(bwt_str == "TEXYDST.E.IXIXIXXSSMPPS.B..E.S.EUSFXDIIOIIIT");
        assert!(bwt.ptr == 29);
    }
}
