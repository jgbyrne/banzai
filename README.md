## banzai

**banzai** is a pure Rust bzip2 encoder. It is currently pre-alpha software and breaks on most non-trivial inputs.


This is original libre software. However, implementation guidance was derived from several free-software sources. 

The suffix array construction algorithm used in **banzai** is [SA-IS](https://ieeexplore.ieee.org/document/5582081), which was developed by Ge Nong, Sen Zhang, and Wai Hong Chan. Guidance for implementing SA-IS was derived from Yuta Mori's [sais](https://sites.google.com/site/yuta256/sais) and burntsushi's [suffix](https://github.com/BurntSushi/suffix).

The implementation of Huffman coding used in **banzai** takes heavy inspiration from the reference implementation of [bzip2](https://gitlab.com/bzip2/bzip2/), originally authored by Julian Seward, currently maintained by Micah Snyder.

Finally, the unofficial [bzip2 Format Specification](https://github.com/dsnet/compress/blob/master/doc/bzip2-format.pdf) written by Joe Tsai was extremely helpful when it came to the specifics of the bzip2 binary format.
