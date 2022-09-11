## banzai

**banzai** is a pure Rust bzip2 encoder. It is currently pre-alpha software.

### Command Line Usage

    banzai <file_to_encode>

Compresses and writes to `file_to_encode.bz2`.

### Library Usage

```rust
fn encode(input: I, writer: io::BufWriter<W>, level: usize) -> io::Result<()>
where
    I: convert::AsRef<[u8]>,
    W: io::Write
```

Call `encode` with a reference to an input buffer and a `BufWriter`. The final parameter is `level`, which is a number between `1` and `9` inclusive, which corresponds to the block size (block size is `level * 100_000` bytes). The typical default is `9`.

### Acknowledgements

This is original libre software. However, implementation guidance was derived from several free-software sources. 

The suffix array construction algorithm used in **banzai** is [SA-IS](https://ieeexplore.ieee.org/document/5582081), which was developed by Ge Nong, Sen Zhang, and Wai Hong Chan. Guidance for implementing SA-IS was derived from Yuta Mori's [sais](https://sites.google.com/site/yuta256/sais) and burntsushi's [suffix](https://github.com/BurntSushi/suffix).

The implementation of Huffman coding used in **banzai** takes heavy inspiration from the reference implementation of [bzip2](https://gitlab.com/bzip2/bzip2/), originally authored by Julian Seward, currently maintained by Micah Snyder.

Finally, the unofficial [bzip2 Format Specification](https://github.com/dsnet/compress/blob/master/doc/bzip2-format.pdf) written by Joe Tsai was extremely helpful when it came to the specifics of the bzip2 binary format.
