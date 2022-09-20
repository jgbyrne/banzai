## bnz

**bnz** is the command-line interface to **banzai**. The interface is deliberately similar to `bzip2(1)`.

You can install **bnz** with cargo: `cargo install bnz`

To compress a file `file_to_encode`, run:

```
bnz file_to_encode
```

This deletes `file_to_encode` and writes a new file `file_to_encode.bz2`.

For full options and usage guidance, run `bnz --help`.
