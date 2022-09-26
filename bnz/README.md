## bnz

**bnz** is a command-line utility for compressing files into the bzip2 format. It is implemented as a thin wrapper around **banzai**, a bzip2 encoding library written entirely in safe Rust. The interface is deliberately similar to `bzip2(1)`.

You can install **bnz** with cargo: `cargo install bnz`

To compress a file `file_to_encode`, run:

```
bnz file_to_encode
```

This deletes `file_to_encode` and writes a new file `file_to_encode.bz2`.

For full options and usage guidance, run `bnz --help`.
