## banzai

**banzai** is a bzip2 encoder, written entirely in safe Rust. It is currently alpha software, which means that it is not battle-hardened and is not guaranteed to perform well and not eat your data. That's not to say, however, that I don't care about performance or reliability - bug reports are warmly appreciated! In the long term I would like to get this library to a state where it can be relied upon in production software.

To use **banzai** as a command-line tool with a similar interface to `bzip(1)`, install **bnz** through cargo.

This library is linear-time in the size of the input, and has no usage of `unsafe`. When it is more mature these features should make it a good choice for safety-critical applications.

**banzai** currently uses a near-identical method of choosing Huffman trees to the reference implementation and therefore achieves very similar compression ratios. Compared to the reference implementation, **banzai** has worse average runtime but better worst-case runtime. This is because of the different algorithms used to compute the Burrows-Wheeler Transform. The choice of algorithm used in **banzai** is *SA-IS*, which computes a [suffix array](https://en.wikipedia.org/wiki/Suffix_array) in linear time. Since bzip2 uses a 'wrap-around' version of the BWT, we are obliged to compute the suffix array of the input concatenated with itself. I intend to investigate ways in which the redundancy inherent to inputs of this form can be exploited to optimise suffix array construction.

This library does not (currently) include a decompressor. Paolo Barbolini's [bzip2-rs](https://crates.io/crates/bzip2-rs) offers a pure Rust bzip2 decompressor, though I have not used it myself and cannot vouch for its quality.

### Interface

```rust
fn encode(reader: R, writer: io::BufWriter<W>, level: usize) -> io::Result<usize>
where
    R: io::BufRead,
    W: io::Write
```

Call `encode` with a reference to an input buffer and a `BufWriter`. The final parameter is `level`, which is a number between `1` and `9` inclusive, which corresponds to the block size (block size is `level * 100_000` bytes). The typical default is `9`. Returns the number of input bytes encoded.

### Safety

**banzai** is written entirely in safe Rust. This is a deliberate choice which will, in future, make **banzai** a good choice for applications where memory-safety is of paramount importance. However, this decisions comes with some performance costs. Experiments suggest that **banzai** could be approximately 10% faster if the extremely hot `Index` impls on `Data` and `Array` in `bwt.rs` were changed to be unchecked. In the future such performance boosts may be made available to consumers of the library behind a feature gate.

### Acknowledgements

This is original libre software. However, implementation guidance was derived from several free-software sources. 

The suffix array construction algorithm used in **banzai** is [SA-IS](https://ieeexplore.ieee.org/document/5582081), which was developed by Ge Nong, Sen Zhang, and Wai Hong Chan. Guidance for implementing SA-IS was derived from Yuta Mori's [sais](https://sites.google.com/site/yuta256/sais) and burntsushi's [suffix](https://github.com/BurntSushi/suffix).

The implementation of Huffman coding used in **banzai** takes heavy inspiration from the reference implementation of [bzip2](https://gitlab.com/bzip2/bzip2/), originally authored by Julian Seward, currently maintained by Micah Snyder.

Finally, the unofficial [bzip2 Format Specification](https://github.com/dsnet/compress/blob/master/doc/bzip2-format.pdf) written by Joe Tsai was extremely helpful when it came to the specifics of the bzip2 binary format.
