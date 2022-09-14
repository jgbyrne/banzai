## banzai

```
cargo install banzai
```

**banzai** is a pure Rust bzip2 encoder. It is currently pre-alpha software, which means that it has undergone a very limited amount of testing and should not be relied upon to perform well and not eat your data. That's not to say, however, that I don't care about performance or reliability - bug reports are warmly appreciated! In the long term I would like to get this library to a state where it can be relied upon in production software.

This library is linear-time in the size of the input, and has no usage of `unsafe`. When it is more mature these features should make it a good choice for safety-critical applications.

In general, **banzai** achieves similar compression ratios to the reference implementation. However, the runtime tends to be approximately twice as long. I believe this is because the runtime is dominated by the Burrows-Wheeler Transform. Since bzip2 uses a 'wrap-around' version of the BWT, **banzai** is obliged to compute the suffix array of the input concatenated with itself. I intend to investigate ways in which the redundancy inherent to inputs of this form can be exploited to optimise suffix array construction.

This library does not (currently) include a decompressor. Paolo Barbolini's [bzip2-rs](https://crates.io/crates/bzip2-rs) offers a pure Rust bzip2 decompressor, though I have not used it myself and cannot vouch for its quality.

### Command Line Usage

    banzai <file_to_encode>

Compresses and writes to `file_to_encode.bz2`.

### Library Usage

```rust
fn encode(input: I, writer: io::BufWriter<W>, level: usize) -> io::Result<usize>
where
    I: convert::AsRef<[u8]>,
    W: io::Write
```

Call `encode` with a reference to an input buffer and a `BufWriter`. The final parameter is `level`, which is a number between `1` and `9` inclusive, which corresponds to the block size (block size is `level * 100_000` bytes). The typical default is `9`. Returns the number of input bytes encoded.

### Acknowledgements

This is original libre software. However, implementation guidance was derived from several free-software sources. 

The suffix array construction algorithm used in **banzai** is [SA-IS](https://ieeexplore.ieee.org/document/5582081), which was developed by Ge Nong, Sen Zhang, and Wai Hong Chan. Guidance for implementing SA-IS was derived from Yuta Mori's [sais](https://sites.google.com/site/yuta256/sais) and burntsushi's [suffix](https://github.com/BurntSushi/suffix).

The implementation of Huffman coding used in **banzai** takes heavy inspiration from the reference implementation of [bzip2](https://gitlab.com/bzip2/bzip2/), originally authored by Julian Seward, currently maintained by Micah Snyder.

Finally, the unofficial [bzip2 Format Specification](https://github.com/dsnet/compress/blob/master/doc/bzip2-format.pdf) written by Joe Tsai was extremely helpful when it came to the specifics of the bzip2 binary format.
