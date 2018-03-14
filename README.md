[![Build Status](https://travis-ci.org/cjm00/dedup.svg?branch=master)](https://travis-ci.org/cjm00/dedup)
[![Build status](https://ci.appveyor.com/api/projects/status/ricpuv3a8q3vep4c/branch/master?svg=true)](https://ci.appveyor.com/project/cjm00/dedup)

A better deduplicator written in Rust.

Basic usage: `dedup <INPUT> [-o <OUTPUTFILE>]`

Run `dedup --help` for usage and command line options.

To run the benchmark run `python benchsuite/benchrunner`. This will download a large (400MB+) text file to use as a benchmark case.

Feature requests and bug reports are always welcome! Please raise them as an issue in this Github repository.