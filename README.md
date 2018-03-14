[![Build Status](https://travis-ci.org/cjm00/dedup.svg?branch=master)](https://travis-ci.org/cjm00/dedup)
[![Build status](https://ci.appveyor.com/api/projects/status/ricpuv3a8q3vep4c/branch/master?svg=true)](https://ci.appveyor.com/project/cjm00/dedup)

A better deduplicator written in Rust.

Basic usage: `dedup <INPUT> [-o <OUTPUTFILE>]`

Run `dedup --help` to see:
```
USAGE:
    dedup.exe [FLAGS] [OPTIONS] [INPUT]

FLAGS:
    -l, --count-lines        If flag is set only print the number of unique entries found.
        --mmap               Enables use of memory mapped files. This is enabled by default.
        --no-mmap            Prohibits usage of memory mapped files. This will slow down the deduplication process
                             significantly!
    -z, --zero-terminated    Specifies that entries should be intepreted as being separated by a null byte rather than a
                             newline.
    -h, --help               Prints help information
    -V, --version            Prints version information

OPTIONS:
    -o, --output <OUTPUT>
        --terminator <TERMINATOR>    Specifies the single-byte pattern to separate entries by. Default is newline.
                                     [default: \n]

ARGS:
    <INPUT>    Specifies the input file to read from. Omit or supply '-' to read from stdin.
```

To run the benchmark run `python benchsuite/benchrunner`. This will download a large (400MB+) text file to use as a benchmark case.

Feature requests and bug reports are always welcome! Please raise them as an issue in this Github repository.